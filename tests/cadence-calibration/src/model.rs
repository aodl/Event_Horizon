use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};

use crate::{
    measurement::{read_measurements, validate_measurements},
    types::{
        CostCalibration, FixedTierEconomics, FundingAssumptions, MeasurementSet, Mode, ModelConfig,
        Policy, ProductiveEndowmentBacking, SimulationResult, SimulationSet, Thresholds, Workload,
    },
    TRILLION,
};

const DAY: u64 = 86_400;
const YEAR_DAYS: u128 = 365;
const ICP_E8S: u128 = 100_000_000;
const FUNDING_MAINTENANCE_SECONDS: u64 = 3_600;
const PRICING_MAINTENANCE_SECONDS: u64 = DAY;

#[derive(Clone)]
struct SimState {
    now: u64,
    balance: u128,
    minimum_balance: u128,
    mode: Mode,
    next_poll: Option<u64>,
    next_funding_maintenance: u64,
    next_pricing_maintenance: u64,
    backlog: u128,
    specific_matching_backlog: u128,
    block_milli_remainder: u128,
    specific_matching_basis_remainder: u128,
    specific_subscriber_milli_remainder: u128,
    overlap_notification_remainder: u128,
    spent: u128,
    ordinary_polling_spent: u128,
    maintenance_spent: u128,
    funded: u128,
    funding_maintenance_runs: u64,
    pricing_maintenance_runs: u64,
    global_notifications: u128,
    specific_notifications: u128,
    coalesced_notification_dispatches: u128,
    seconds_in_modes: [u64; 6],
    transitions: Vec<u64>,
    first_downgrade: Option<u64>,
    first_reserve: Option<u64>,
    maximum_backlog: u128,
    maximum_pre_poll: u128,
    maximum_post_poll: u128,
    downgrade_backlog: Option<u128>,
    reserve_backlog: Option<u128>,
    reserve_generated: u128,
    exhausted: bool,
}

#[derive(Clone, Copy)]
struct Batch {
    elapsed: u64,
    polls: u64,
    generated: u128,
    generated_specific_matching: u128,
    processed: u128,
    processed_specific_matching: u128,
    global_notifications: u128,
    specific_notifications: u128,
    coalesced_notification_dispatches: u128,
    maximum_pre_poll: u128,
    maximum_post_poll: u128,
    burn: u128,
    block_remainder: u128,
    specific_matching_remainder: u128,
    specific_subscriber_remainder: u128,
    overlap_notification_remainder: u128,
}

#[derive(Clone, Copy)]
struct SimulationContext<'a> {
    calibration: &'a CostCalibration,
    assumptions: &'a FundingAssumptions,
    seed: u64,
    continuous_interval_seconds: u64,
}

pub fn run(
    input: &Path,
    output: &Path,
    seed: u64,
    config_path: Option<&Path>,
) -> Result<SimulationSet> {
    fs::create_dir_all(output).with_context(|| format!("create {}", output.display()))?;
    let measurements = read_measurements(input)?;
    let calibrations = calibrate(&measurements)?;
    let calibration = calibrations
        .iter()
        .find(|item| item.architecture == "canonical_icp_shared_scan")
        .cloned()
        .ok_or_else(|| anyhow!("missing canonical ICP calibration"))?;
    let model_config = match config_path {
        Some(path) => read_model_config(path)?,
        None => default_model_config(),
    };
    let assumptions = model_config.funding_assumptions.clone();
    let policies = model_config.policies.clone();
    for policy in &policies {
        policy
            .thresholds
            .validate()
            .map_err(|error| anyhow!("policy {}: {error}", policy.id))?;
    }
    let configured_workloads = model_config.workloads.clone();
    let unavailable_architectures: Vec<String> = configured_workloads
        .iter()
        .filter(|workload| {
            !calibrations
                .iter()
                .any(|item| item.architecture == workload.architecture)
        })
        .map(|workload| format!("{} ({})", workload.id, workload.architecture))
        .collect();
    let workloads: Vec<Workload> = configured_workloads
        .into_iter()
        .filter(|workload| {
            calibrations
                .iter()
                .any(|item| item.architecture == workload.architecture)
        })
        .collect();
    let mut results = Vec::new();
    let funding_profiles = [
        "no_incoming_funding",
        "regular_daily",
        "regular_weekly",
        "regular_monthly",
        "irregular",
        "seven_day_interruption",
        "thirty_day_interruption",
        "ninety_day_interruption",
        "delayed_payout_and_recovery",
        "low_yield_2_8_percent",
        "zero_effective_yield",
    ];
    let current = policies
        .iter()
        .find(|policy| policy.id == "current")
        .ok_or_else(|| anyhow!("model configuration has no `current` policy"))?;
    let expected = workloads
        .iter()
        .find(|workload| workload.id == "expected")
        .ok_or_else(|| anyhow!("model configuration has no `expected` workload"))?;
    for profile in funding_profiles {
        for horizon in [30, 90, 365] {
            results.push(simulate_for(
                current,
                expected,
                &calibrations,
                &assumptions,
                seed,
                model_config.continuous_effective_poll_interval_seconds,
                "funding_profile",
                profile,
                horizon,
                current.thresholds.continuous_entry,
            )?);
        }
    }
    for policy in &policies {
        for workload in &workloads {
            for horizon in [30, 90, 365] {
                results.push(simulate_for(
                    policy,
                    workload,
                    &calibrations,
                    &assumptions,
                    seed,
                    model_config.continuous_effective_poll_interval_seconds,
                    "candidate_workload",
                    "regular_weekly",
                    horizon,
                    policy.thresholds.continuous_entry,
                )?);
            }
        }
    }
    let architecture_workloads: Vec<&Workload> = workloads
        .iter()
        .filter(|workload| workload.id == "expected" || workload.id == "expected_generic")
        .collect();
    for policy in &policies {
        for workload in &architecture_workloads {
            for mode in [Mode::Economy, Mode::Standard, Mode::Fast, Mode::VeryFast] {
                results.push(simulate_for(
                    policy,
                    workload,
                    &calibrations,
                    &assumptions,
                    seed,
                    model_config.continuous_effective_poll_interval_seconds,
                    &format!("entry_runway_{}", mode_id(mode)),
                    "no_incoming_funding",
                    365,
                    policy.thresholds.entry(mode),
                )?);
            }
        }
    }
    for policy in &policies {
        for &initial_cycles in &model_config.equal_starting_balances_cycles {
            for &(profile, horizon) in &[("no_incoming_funding", 90), ("regular_weekly", 365)] {
                results.push(simulate_for(
                    policy,
                    expected,
                    &calibrations,
                    &assumptions,
                    seed,
                    model_config.continuous_effective_poll_interval_seconds,
                    "equal_balance",
                    profile,
                    horizon,
                    initial_cycles,
                )?);
            }
        }
    }
    for policy in &policies {
        for &interval in &model_config.continuous_effective_poll_intervals_seconds {
            for profile in ["no_incoming_funding", "regular_weekly"] {
                results.push(simulate_for(
                    policy,
                    expected,
                    &calibrations,
                    &assumptions,
                    seed,
                    interval,
                    "poll_rate_sensitivity",
                    profile,
                    365,
                    policy.thresholds.continuous_entry,
                )?);
            }
        }
    }
    for policy in &policies {
        for profile in [
            "seven_day_interruption",
            "thirty_day_interruption",
            "ninety_day_interruption",
            "delayed_payout_and_recovery",
        ] {
            results.push(simulate_for(
                policy,
                expected,
                &calibrations,
                &assumptions,
                seed,
                model_config.continuous_effective_poll_interval_seconds,
                "interruption_comparison",
                profile,
                365,
                policy.thresholds.continuous_entry,
            )?);
        }
    }

    let mut limitations = vec![
        "Measured PocketIC costs are extrapolated linearly; the simulator does not claim mainnet instruction or subnet-cost equivalence.".into(),
        format!("Continuous has no protocol delay. The model evaluates completed-poll intervals {:?} seconds; all are assumptions until production observations establish an effective rate.", model_config.continuous_effective_poll_intervals_seconds),
        "Traffic is deterministic. Global notifications are charged once per global subscriber for each nonempty poll; specific notifications use the configured matching-block and subscribers-per-match assumptions, capped once per subscriber per poll.".into(),
        "Each workload's blocks-per-poll capacity is a modelling assumption. Completing a 4,096-block PocketIC backlog does not establish a hard protocol or mainnet maximum.".into(),
        "Funding and pricing maintenance costs are residual estimates from integrated scheduler measurements, not independently isolated lane benchmarks.".into(),
        "Funding inputs are assumptions. Productive principal remains staked in Jupiter; only synthetic maturity-derived payouts are credited as liquid cycles.".into(),
        "The model distinguishes average sustainability from liquidity timing, but does not predict ICP price, XDR rate, maturity, conversion execution, or subnet congestion.".into(),
    ];
    if !unavailable_architectures.is_empty() {
        limitations.push(format!(
            "Input measurements lacked architecture calibration for {}; those configured workloads were explicitly omitted.",
            unavailable_architectures.join(", ")
        ));
    }
    if calibration.funding_maintenance_cycles.is_none()
        || calibration.pricing_maintenance_cycles.is_none()
    {
        limitations.push("Input measurements lacked integrated scheduler evidence; offline maintenance cost is zero and reported as unmeasured for this dataset.".into());
    }
    let mut fixed_tier_economics = Vec::new();
    for workload in &workloads {
        let calibration = calibrations
            .iter()
            .find(|c| c.architecture == workload.architecture)
            .expect("filtered architecture");
        for mode in Mode::ACTIVE {
            let intervals = if mode == Mode::Continuous {
                vec![1, 2, 5, 10]
            } else {
                vec![mode.delay_seconds(1).expect("active mode")]
            };
            for interval in intervals {
                fixed_tier_economics.push(fixed_tier_economics_for(
                    workload,
                    calibration,
                    &assumptions,
                    mode,
                    interval,
                    7,
                )?);
            }
        }
    }
    let set = SimulationSet {
        schema_version: 3,
        source_measurements: input.display().to_string(),
        seed,
        calibration,
        architecture_calibrations: calibrations,
        funding_assumptions: assumptions,
        policies,
        workloads,
        continuous_effective_poll_interval_seconds: model_config
            .continuous_effective_poll_interval_seconds,
        continuous_effective_poll_intervals_seconds: model_config
            .continuous_effective_poll_intervals_seconds
            .clone(),
        equal_starting_balances_cycles: model_config.equal_starting_balances_cycles.clone(),
        continuous_runway_targets_days: model_config.continuous_runway_targets_days,
        lower_tier_runway_targets_days: model_config.lower_tier_runway_targets_days,
        fixed_tier_economics,
        results,
        limitations,
    };
    fs::write(
        output.join("simulations.json"),
        serde_json::to_vec_pretty(&set)?,
    )?;
    copy_measurement(input, &output.join("measurements.json"))?;
    if let Some(parent) = input.parent() {
        let source_csv = parent.join("measurements.csv");
        match fs::metadata(&source_csv) {
            Ok(metadata) if metadata.is_file() => {
                copy_measurement(&source_csv, &output.join("measurements.csv"))?;
            }
            Ok(_) => bail!("measurement CSV is not a file: {}", source_csv.display()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("inspect {}", source_csv.display()))
            }
        }
    }
    write_threshold_csv(&set, output)?;
    Ok(set)
}

// Resolve existing aliases before opening a destination for writing. On Unix,
// inode identity also catches hard links, which canonical paths cannot detect.
fn copy_measurement(source: &Path, destination: &Path) -> Result<()> {
    let context = || format!("copy {} to {}", source.display(), destination.display());
    let source_path = fs::canonicalize(source).with_context(context)?;
    match fs::canonicalize(destination) {
        Ok(destination_path) => {
            if source_path == destination_path {
                return Ok(());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let source_metadata = fs::metadata(source).with_context(context)?;
                let destination_metadata = fs::metadata(destination).with_context(context)?;
                if source_metadata.dev() == destination_metadata.dev()
                    && source_metadata.ino() == destination_metadata.ino()
                {
                    return Ok(());
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(context),
    }
    fs::copy(source, destination).with_context(context)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn simulate_for(
    policy: &Policy,
    workload: &Workload,
    calibrations: &[CostCalibration],
    assumptions: &FundingAssumptions,
    seed: u64,
    continuous_interval_seconds: u64,
    simulation_kind: &str,
    funding_profile: &str,
    horizon_days: u64,
    initial_cycles: u128,
) -> Result<SimulationResult> {
    let calibration = calibrations
        .iter()
        .find(|item| item.architecture == workload.architecture)
        .ok_or_else(|| {
            anyhow!(
                "workload {} requires unavailable {} calibration",
                workload.id,
                workload.architecture
            )
        })?;
    let context = SimulationContext {
        calibration,
        assumptions,
        seed,
        continuous_interval_seconds,
    };
    simulate(
        policy,
        workload,
        &context,
        simulation_kind,
        funding_profile,
        horizon_days,
        initial_cycles,
    )
}

pub fn calibrate(set: &MeasurementSet) -> Result<Vec<CostCalibration>> {
    validate_measurements(set)?;
    let canonical_baseline: Vec<u128> = set
        .scenarios
        .iter()
        .filter(|item| {
            item.experiment == "A_baseline_polling"
                && item.architecture == "canonical_icp_shared_scan"
                && !item.maintenance_included
        })
        .filter_map(|item| item.median_cycles_per_poll)
        .collect();
    if canonical_baseline.is_empty() {
        bail!("no completed canonical ICP baseline polling scenarios")
    }
    let canonical_empty_poll_cycles = median(canonical_baseline);

    let global_zero = fanout(set, 0)?;
    let global_high = set
        .scenarios
        .iter()
        .filter(|item| {
            item.experiment == "C_notification_fanout"
                && item.subscriber_kind == "global"
                && item.subscriber_count > 0
        })
        .max_by_key(|item| item.subscriber_count)
        .ok_or_else(|| anyhow!("no nonzero global fan-out scenario"))?;
    let cycles_per_subscriber_notification = global_high
        .net_cycles_consumed
        .saturating_sub(global_zero.net_cycles_consumed)
        / u128::from(global_high.subscriber_count);
    let economy = set.scenarios.iter().find(|item| {
        item.experiment == "A_baseline_polling"
            && item.polling_mode == Some(Mode::Economy)
            && item.maintenance_included
    });
    let integrated = set
        .scenarios
        .iter()
        .find(|item| item.scenario_id == "integrated_fast_all_scheduler_lanes");
    let funding_maintenance_cycles = economy
        .and_then(|item| item.median_cycles_per_poll)
        .map(|value| value.saturating_sub(canonical_empty_poll_cycles));
    let pricing_maintenance_cycles = integrated.and_then(|item| {
        let polling = canonical_empty_poll_cycles.saturating_mul(u128::from(item.completed_polls));
        let combined = item.net_cycles_consumed.saturating_sub(polling);
        funding_maintenance_cycles.map(|funding| combined.saturating_sub(funding))
    });

    let mut calibrations = Vec::new();
    for architecture in [
        "canonical_icp_shared_scan",
        "generic_icrc3_separate_admission_scan",
    ] {
        let empty_poll_cycles = if architecture == "canonical_icp_shared_scan" {
            canonical_empty_poll_cycles
        } else {
            let Some(idle) = set.scenarios.iter().find(|item| {
                item.experiment == "B_ledger_volume"
                    && item.architecture == architecture
                    && item.traffic_profile == "idle_0_blocks_per_second"
                    && item.completed_polls > 0
            }) else {
                continue;
            };
            idle.net_cycles_consumed / u128::from(idle.completed_polls)
        };
        let mut per_block_samples = Vec::new();
        for item in set.scenarios.iter().filter(|item| {
            item.experiment == "B_ledger_volume"
                && item.architecture == architecture
                && item.processed_blocks > 0
                && !item.traffic_profile.starts_with("archive_prefix")
        }) {
            let fixed = empty_poll_cycles.saturating_mul(u128::from(item.completed_polls));
            let variable = item.net_cycles_consumed.saturating_sub(fixed);
            per_block_samples.push(variable / u128::from(item.processed_blocks));
        }
        if per_block_samples.is_empty() {
            bail!("no completed {architecture} block-processing scenarios")
        }
        let cycles_per_processed_block = median(per_block_samples);
        let page_scenario = set
            .scenarios
            .iter()
            .filter(|item| {
                item.experiment == "B_ledger_volume"
                    && item.architecture == architecture
                    && item.ledger_pages > item.completed_polls
            })
            .max_by_key(|item| item.processed_blocks)
            .ok_or_else(|| anyhow!("no multi-page {architecture} scenario"))?;
        let expected = empty_poll_cycles
            .saturating_mul(u128::from(page_scenario.completed_polls))
            .saturating_add(
                cycles_per_processed_block
                    .saturating_mul(u128::from(page_scenario.processed_blocks)),
            );
        let additional_pages = page_scenario
            .ledger_pages
            .saturating_sub(page_scenario.completed_polls);
        let cycles_per_ledger_page = page_scenario.net_cycles_consumed.saturating_sub(expected)
            / u128::from(additional_pages.max(1));
        let largest_completed_backlog = set
            .scenarios
            .iter()
            .filter(|item| item.complete && item.architecture == architecture)
            .map(|item| item.processed_blocks)
            .max()
            .unwrap_or_default();
        if empty_poll_cycles == 0
            || cycles_per_processed_block == 0
            || cycles_per_subscriber_notification == 0
            || largest_completed_backlog == 0
        {
            bail!("{architecture} calibration produced a zero required component")
        }
        calibrations.push(CostCalibration {
            architecture: architecture.into(),
            empty_poll_cycles,
            cycles_per_processed_block,
            cycles_per_ledger_page,
            cycles_per_subscriber_notification,
            funding_maintenance_cycles,
            pricing_maintenance_cycles,
            largest_completed_backlog,
            sources: vec![
                format!("Idle polling cost for {architecture}"),
                format!("Median residual block cost for {architecture}"),
                format!("Additional-page residual from {}", page_scenario.scenario_id),
                format!(
                    "Settled global fan-out slope between 0 and {} subscribers",
                    global_high.subscriber_count
                ),
                "Funding residual from maintenance-inclusive Economy minus isolated canonical polling".into(),
                "Pricing residual from integrated all-lane Fast minus isolated polling and estimated funding maintenance".into(),
            ],
        });
    }
    Ok(calibrations)
}

fn fanout(set: &MeasurementSet, count: u64) -> Result<&crate::types::Measurement> {
    set.scenarios
        .iter()
        .find(|item| {
            item.experiment == "C_notification_fanout"
                && item.subscriber_kind == "global"
                && item.subscriber_count == count
        })
        .ok_or_else(|| anyhow!("missing global fan-out scenario for {count} subscribers"))
}

pub fn next_mode(policy: &Thresholds, current: Mode, balance: u128) -> Mode {
    if balance < TRILLION {
        return Mode::ReserveProtection;
    }
    let fastest = fastest_enterable(policy, balance);
    if rank(fastest) > rank(current) {
        return fastest;
    }
    if current == Mode::ReserveProtection {
        return fastest;
    }
    if balance >= policy.exit(current) {
        return current;
    }
    let maximum = match current {
        Mode::Continuous => Mode::VeryFast,
        Mode::VeryFast => Mode::Fast,
        Mode::Fast => Mode::Standard,
        Mode::Standard => Mode::Economy,
        Mode::Economy | Mode::ReserveProtection => return Mode::ReserveProtection,
    };
    fallback_from(policy, maximum, balance)
}

fn fastest_enterable(policy: &Thresholds, balance: u128) -> Mode {
    for mode in [
        Mode::Continuous,
        Mode::VeryFast,
        Mode::Fast,
        Mode::Standard,
        Mode::Economy,
    ] {
        if balance >= policy.entry(mode) {
            return mode;
        }
    }
    Mode::ReserveProtection
}

fn fallback_from(policy: &Thresholds, maximum: Mode, balance: u128) -> Mode {
    let candidate = fastest_enterable(policy, balance);
    if rank(candidate) <= rank(maximum) && candidate != Mode::ReserveProtection {
        return candidate;
    }
    for mode in [Mode::VeryFast, Mode::Fast, Mode::Standard, Mode::Economy] {
        if rank(mode) <= rank(maximum) && balance >= policy.exit(mode) {
            return mode;
        }
    }
    Mode::ReserveProtection
}

const fn rank(mode: Mode) -> usize {
    match mode {
        Mode::ReserveProtection => 0,
        Mode::Economy => 1,
        Mode::Standard => 2,
        Mode::Fast => 3,
        Mode::VeryFast => 4,
        Mode::Continuous => 5,
    }
}

const fn mode_id(mode: Mode) -> &'static str {
    match mode {
        Mode::ReserveProtection => "reserve_protection",
        Mode::Economy => "economy",
        Mode::Standard => "standard",
        Mode::Fast => "fast",
        Mode::VeryFast => "very_fast",
        Mode::Continuous => "continuous",
    }
}

fn simulate(
    policy: &Policy,
    workload: &Workload,
    context: &SimulationContext<'_>,
    simulation_kind: &str,
    funding_profile: &str,
    horizon_days: u64,
    initial_cycles: u128,
) -> Result<SimulationResult> {
    let end = horizon_days
        .checked_mul(DAY)
        .ok_or_else(|| anyhow!("simulation horizon overflow"))?;
    let funding = funding_schedule(
        funding_profile,
        end,
        context.assumptions,
        workload,
        context.seed,
    )?;
    let mut funding_index = 0_usize;
    let initial_mode = next_mode(&policy.thresholds, Mode::ReserveProtection, initial_cycles);
    let mut state = SimState {
        now: 0,
        balance: initial_cycles,
        minimum_balance: initial_cycles,
        mode: initial_mode,
        next_poll: initial_mode
            .delay_seconds(context.continuous_interval_seconds)
            .map(|delay| delay.min(end)),
        next_funding_maintenance: FUNDING_MAINTENANCE_SECONDS,
        next_pricing_maintenance: PRICING_MAINTENANCE_SECONDS,
        backlog: u128::from(workload.initial_backlog_blocks),
        specific_matching_backlog: u128::from(workload.initial_backlog_blocks)
            .saturating_mul(u128::from(workload.specific_matching_blocks_basis_points))
            / 10_000,
        block_milli_remainder: 0,
        specific_matching_basis_remainder: 0,
        specific_subscriber_milli_remainder: 0,
        overlap_notification_remainder: 0,
        spent: 0,
        ordinary_polling_spent: 0,
        maintenance_spent: 0,
        funded: 0,
        funding_maintenance_runs: 0,
        pricing_maintenance_runs: 0,
        global_notifications: 0,
        specific_notifications: 0,
        coalesced_notification_dispatches: 0,
        seconds_in_modes: [0; 6],
        transitions: Vec::new(),
        first_downgrade: None,
        first_reserve: (initial_mode == Mode::ReserveProtection).then_some(0),
        maximum_backlog: u128::from(workload.initial_backlog_blocks),
        maximum_pre_poll: 0,
        maximum_post_poll: 0,
        downgrade_backlog: None,
        reserve_backlog: (initial_mode == Mode::ReserveProtection)
            .then_some(u128::from(workload.initial_backlog_blocks)),
        reserve_generated: 0,
        exhausted: initial_cycles == 0,
    };

    while state.now < end {
        let next_funding = funding
            .get(funding_index)
            .map(|(time, _)| *time)
            .unwrap_or(u64::MAX);
        let next_poll = state.next_poll.unwrap_or(u64::MAX);
        let next_maintenance = state
            .next_funding_maintenance
            .min(state.next_pricing_maintenance);
        if next_funding <= next_poll && next_funding <= next_maintenance && next_funding <= end {
            let elapsed = next_funding - state.now;
            accrue_without_poll(&mut state, workload, elapsed);
            state.now = next_funding;
            while funding_index < funding.len() && funding[funding_index].0 == state.now {
                let amount = funding[funding_index].1;
                state.balance = state.balance.saturating_add(amount);
                state.funded = state.funded.saturating_add(amount);
                funding_index += 1;
            }
            change_mode(&mut state, policy, context.continuous_interval_seconds);
            continue;
        }
        if next_maintenance <= next_poll && next_maintenance <= end {
            let elapsed = next_maintenance - state.now;
            accrue_without_poll(&mut state, workload, elapsed);
            state.now = next_maintenance;
            let mut maintenance_burn = 0_u128;
            if state.next_funding_maintenance == state.now {
                maintenance_burn = maintenance_burn.saturating_add(
                    context
                        .calibration
                        .funding_maintenance_cycles
                        .unwrap_or_default(),
                );
                state.funding_maintenance_runs = state.funding_maintenance_runs.saturating_add(1);
                state.next_funding_maintenance = state
                    .next_funding_maintenance
                    .saturating_add(FUNDING_MAINTENANCE_SECONDS);
            }
            if state.next_pricing_maintenance == state.now {
                maintenance_burn = maintenance_burn.saturating_add(
                    context
                        .calibration
                        .pricing_maintenance_cycles
                        .unwrap_or_default(),
                );
                state.pricing_maintenance_runs = state.pricing_maintenance_runs.saturating_add(1);
                state.next_pricing_maintenance = state
                    .next_pricing_maintenance
                    .saturating_add(PRICING_MAINTENANCE_SECONDS);
            }
            state.maintenance_spent = state.maintenance_spent.saturating_add(maintenance_burn);
            apply_burn(&mut state, maintenance_burn);
            change_mode(&mut state, policy, context.continuous_interval_seconds);
            continue;
        }
        if next_poll <= end {
            let interval = state
                .mode
                .delay_seconds(context.continuous_interval_seconds)
                .unwrap_or(DAY)
                .max(1);
            let limit = next_funding.min(next_maintenance).min(end);
            let external_event_at_limit =
                limit <= end && (limit == next_funding || limit == next_maintenance);
            let possible = if external_event_at_limit {
                limit.saturating_sub(next_poll).div_ceil(interval)
            } else {
                1 + limit.saturating_sub(next_poll) / interval
            };
            let mut batch_count = possible.max(1);
            let headroom = state
                .balance
                .saturating_sub(policy.thresholds.exit(state.mode));
            let full = project_batch(
                &state,
                workload,
                context.calibration,
                next_poll,
                batch_count,
                interval,
            )?;
            if full.burn > headroom && batch_count > 1 {
                let mut low = 1_u64;
                let mut high = batch_count;
                while low < high {
                    let middle = low + (high - low).div_ceil(2);
                    let projection = project_batch(
                        &state,
                        workload,
                        context.calibration,
                        next_poll,
                        middle,
                        interval,
                    )?;
                    if projection.burn <= headroom {
                        low = middle;
                    } else {
                        high = middle - 1;
                    }
                }
                batch_count = low;
            }
            let batch = project_batch(
                &state,
                workload,
                context.calibration,
                next_poll,
                batch_count,
                interval,
            )?;
            apply_batch(&mut state, batch);
            state.next_poll = Some(state.now.saturating_add(interval));
            change_mode(&mut state, policy, context.continuous_interval_seconds);
            continue;
        }
        let elapsed = end - state.now;
        accrue_without_poll(&mut state, workload, elapsed);
        state.now = end;
    }

    let max_transitions = max_rolling_transitions(&state.transitions, 7 * DAY);
    let funding_shortfall = state.spent.saturating_sub(state.funded);
    let funding_surplus = state.funded.saturating_sub(state.spent);
    let delay = state
        .mode
        .delay_seconds(context.continuous_interval_seconds)
        .unwrap_or(DAY);
    let capacity = u128::from(workload.assumed_blocks_per_poll_capacity);
    let clearance_polls = state.backlog.div_ceil(capacity);
    let projected_latency = u64::try_from(clearance_polls)
        .unwrap_or(u64::MAX)
        .saturating_mul(delay)
        .saturating_add(delay);
    let liquidity_maintained =
        !state.exhausted && state.first_reserve.is_none() && state.balance >= TRILLION;
    let window_funding_covers_expenditure = state.funded >= state.spent;
    Ok(SimulationResult {
        scenario_id: format!(
            "{}_{}_{}_{}_{}d_{}s_{}t",
            simulation_kind,
            policy.id,
            workload.id,
            funding_profile,
            horizon_days,
            context.continuous_interval_seconds,
            initial_cycles / TRILLION,
        ),
        simulation_kind: simulation_kind.into(),
        policy_id: policy.id.clone(),
        funding_profile: funding_profile.into(),
        workload_id: workload.id.clone(),
        horizon_days,
        initial_cycles,
        initial_mode,
        continuous_effective_poll_interval_seconds: context.continuous_interval_seconds,
        total_funding_cycles: state.funded,
        total_projected_expenditure_cycles: state.spent,
        total_ordinary_polling_cycles: state.ordinary_polling_spent,
        total_maintenance_cycles: state.maintenance_spent,
        funding_maintenance_runs: state.funding_maintenance_runs,
        pricing_maintenance_runs: state.pricing_maintenance_runs,
        global_notifications: state.global_notifications,
        specific_notifications: state.specific_notifications,
        coalesced_notification_dispatches: state.coalesced_notification_dispatches,
        ending_cycles: state.balance,
        minimum_cycles: state.minimum_balance,
        seconds_in_modes: state.seconds_in_modes,
        tier_transitions: state.transitions.len() as u64,
        maximum_transitions_in_rolling_seven_days: max_transitions,
        first_downgrade_seconds: state.first_downgrade,
        reserve_protection_seconds: state.first_reserve,
        ending_backlog_blocks: state.backlog,
        maximum_backlog_blocks: state.maximum_backlog,
        maximum_active_pre_poll_pending_blocks: state.maximum_pre_poll,
        maximum_active_post_poll_outstanding_blocks: state.maximum_post_poll,
        outstanding_blocks_at_first_downgrade: state.downgrade_backlog,
        outstanding_blocks_at_first_reserve: state.reserve_backlog,
        additional_reserve_backlog_blocks: state.reserve_generated,
        projected_latency_seconds: projected_latency,
        funding_shortfall_cycles: funding_shortfall,
        funding_surplus_cycles: funding_surplus,
        liquidity_maintained,
        window_funding_covers_expenditure,
        exhausted: state.exhausted,
        continuous_runway_seconds: if initial_mode == Mode::Continuous {
            state.first_downgrade
        } else {
            None
        },
        notes: vec![
            format!("Continuous comparison assumes one completed poll every {} second(s); this is an explicit model input, not a PocketIC or mainnet observation.", context.continuous_interval_seconds),
            format!("Global notifications are coalesced once per global subscriber per nonempty poll. Specific notifications use {} matching-block basis points and {} milli-subscribers per matching block, capped once per subscriber per poll. {} subscribers overlap both registries and are charged as one dispatch when both match.", workload.specific_matching_blocks_basis_points, workload.specific_subscribers_per_matching_block_milli, workload.global_specific_overlap_subscribers),
            "Notifications are charged to Event Horizon only; recipient execution is outside this balance model.".into(),
        ],
    })
}

fn project_batch(
    state: &SimState,
    workload: &Workload,
    calibration: &CostCalibration,
    first_poll: u64,
    polls: u64,
    interval: u64,
) -> Result<Batch> {
    if polls == 0 || interval == 0 {
        bail!("batch requires positive poll count and interval")
    }
    let mut cursor = state.clone();
    let mut total = Batch::zero();
    let mut seen: HashMap<[u128; 6], (u64, Batch)> = HashMap::new();
    let mut index = 0_u64;
    while index < polls {
        // After the first poll, all intervals are equal. Identical backlog and
        // fractional remainders imply an identical future cycle; skip it exactly.
        if index > 0 {
            let key = [
                cursor.backlog,
                cursor.specific_matching_backlog,
                cursor.block_milli_remainder,
                cursor.specific_matching_basis_remainder,
                cursor.specific_subscriber_milli_remainder,
                cursor.overlap_notification_remainder,
            ];
            if let Some((previous_index, previous_total)) = seen.get(&key) {
                let cycle_polls = index - previous_index;
                let repetitions = (polls - index) / cycle_polls;
                if repetitions > 0 {
                    let delta = total.subtract_counts(previous_total)?;
                    total.add_scaled(&delta, repetitions)?;
                    cursor.now = cursor
                        .now
                        .checked_add(
                            interval
                                .checked_mul(cycle_polls)
                                .and_then(|value| value.checked_mul(repetitions))
                                .ok_or_else(|| anyhow!("poll time overflow"))?,
                        )
                        .ok_or_else(|| anyhow!("poll time overflow"))?;
                    index += cycle_polls * repetitions;
                    continue;
                }
            } else {
                seen.insert(key, (index, total));
            }
        }
        let poll_time = first_poll
            .checked_add(
                interval
                    .checked_mul(index)
                    .ok_or_else(|| anyhow!("poll time overflow"))?,
            )
            .ok_or_else(|| anyhow!("poll time overflow"))?;
        let one = project_single_poll(&cursor, workload, calibration, poll_time)?;
        total.add_scaled(&one, 1)?;
        apply_batch(&mut cursor, one);
        index += 1;
    }
    total.elapsed = cursor.now - state.now;
    total.block_remainder = cursor.block_milli_remainder;
    total.specific_matching_remainder = cursor.specific_matching_basis_remainder;
    total.specific_subscriber_remainder = cursor.specific_subscriber_milli_remainder;
    total.overlap_notification_remainder = cursor.overlap_notification_remainder;
    Ok(total)
}

impl Batch {
    const fn zero() -> Self {
        Self {
            elapsed: 0,
            polls: 0,
            generated: 0,
            generated_specific_matching: 0,
            processed: 0,
            processed_specific_matching: 0,
            global_notifications: 0,
            specific_notifications: 0,
            coalesced_notification_dispatches: 0,
            maximum_pre_poll: 0,
            maximum_post_poll: 0,
            burn: 0,
            block_remainder: 0,
            specific_matching_remainder: 0,
            specific_subscriber_remainder: 0,
            overlap_notification_remainder: 0,
        }
    }

    fn subtract_counts(&self, other: &Self) -> Result<Self> {
        let mut result = *self;
        macro_rules! subtract {
            ($field:ident) => {
                result.$field = result
                    .$field
                    .checked_sub(other.$field)
                    .ok_or_else(|| anyhow!("batch cycle underflow"))?;
            };
        }
        subtract!(polls);
        subtract!(generated);
        subtract!(generated_specific_matching);
        subtract!(processed);
        subtract!(processed_specific_matching);
        subtract!(global_notifications);
        subtract!(specific_notifications);
        subtract!(coalesced_notification_dispatches);
        subtract!(burn);
        Ok(result)
    }

    fn add_scaled(&mut self, other: &Self, factor: u64) -> Result<()> {
        macro_rules! add {
            ($field:ident) => {
                self.$field = self
                    .$field
                    .checked_add(
                        other
                            .$field
                            .checked_mul(factor.into())
                            .ok_or_else(|| anyhow!("batch projection overflow"))?,
                    )
                    .ok_or_else(|| anyhow!("batch projection overflow"))?;
            };
        }
        add!(polls);
        add!(generated);
        add!(generated_specific_matching);
        add!(processed);
        add!(processed_specific_matching);
        add!(global_notifications);
        add!(specific_notifications);
        add!(coalesced_notification_dispatches);
        add!(burn);
        self.maximum_pre_poll = self.maximum_pre_poll.max(other.maximum_pre_poll);
        self.maximum_post_poll = self.maximum_post_poll.max(other.maximum_post_poll);
        Ok(())
    }
}

fn project_single_poll(
    state: &SimState,
    workload: &Workload,
    calibration: &CostCalibration,
    poll_time: u64,
) -> Result<Batch> {
    let elapsed = poll_time
        .checked_sub(state.now)
        .ok_or_else(|| anyhow!("poll time reversed"))?;
    let generated_numerator = state
        .block_milli_remainder
        .checked_add(
            u128::from(workload.blocks_per_second_milli)
                .checked_mul(u128::from(elapsed))
                .ok_or_else(|| anyhow!("traffic generation overflow"))?,
        )
        .ok_or_else(|| anyhow!("traffic generation overflow"))?;
    let generated = generated_numerator / 1_000;
    let block_remainder = generated_numerator % 1_000;
    let specific_matching_numerator = state
        .specific_matching_basis_remainder
        .checked_add(
            generated
                .checked_mul(u128::from(workload.specific_matching_blocks_basis_points))
                .ok_or_else(|| anyhow!("specific matching traffic overflow"))?,
        )
        .ok_or_else(|| anyhow!("specific matching traffic overflow"))?;
    let generated_specific_matching = specific_matching_numerator / 10_000;
    let specific_matching_remainder = specific_matching_numerator % 10_000;
    let available = state.backlog.saturating_add(generated);
    let specific_matching_available = state
        .specific_matching_backlog
        .saturating_add(generated_specific_matching);
    let capacity = u128::from(workload.assumed_blocks_per_poll_capacity);
    let processed = available.min(capacity);
    let processed_specific_matching = if processed == available {
        specific_matching_available
    } else if available == 0 {
        0
    } else {
        specific_matching_available
            .checked_mul(processed)
            .ok_or_else(|| anyhow!("specific matching backlog projection overflow"))?
            / available
    };
    let nonempty_polls = u128::from(processed > 0);
    let global_notifications = nonempty_polls
        .checked_mul(u128::from(workload.global_subscribers))
        .ok_or_else(|| anyhow!("notification count overflow"))?;
    let specific_subscriber_numerator = state
        .specific_subscriber_milli_remainder
        .checked_add(
            processed_specific_matching
                .checked_mul(u128::from(
                    workload.specific_subscribers_per_matching_block_milli,
                ))
                .ok_or_else(|| anyhow!("specific notification count overflow"))?,
        )
        .ok_or_else(|| anyhow!("specific notification count overflow"))?;
    let potential_specific_notifications = specific_subscriber_numerator / 1_000;
    let specific_subscriber_remainder = specific_subscriber_numerator % 1_000;
    let specific_notification_cap = nonempty_polls
        .checked_mul(u128::from(workload.specific_subscribers))
        .ok_or_else(|| anyhow!("specific notification cap overflow"))?;
    let specific_notifications = potential_specific_notifications.min(specific_notification_cap);
    let (overlap_notifications, overlap_notification_remainder) =
        if workload.specific_subscribers == 0 {
            (0, 0)
        } else {
            let numerator = state
                .overlap_notification_remainder
                .checked_add(
                    specific_notifications
                        .checked_mul(u128::from(workload.global_specific_overlap_subscribers))
                        .ok_or_else(|| anyhow!("overlap notification count overflow"))?,
                )
                .ok_or_else(|| anyhow!("overlap notification count overflow"))?;
            let divisor = u128::from(workload.specific_subscribers);
            (numerator / divisor, numerator % divisor)
        };
    let coalesced_notification_dispatches = global_notifications
        .checked_add(specific_notifications)
        .and_then(|value| value.checked_sub(overlap_notifications))
        .ok_or_else(|| anyhow!("notification count overflow"))?;
    let pages = processed.div_ceil(256);
    let base_pages = 1;
    let additional_pages = pages.saturating_sub(base_pages);
    let burn = calibration
        .empty_poll_cycles
        .checked_mul(1)
        .and_then(|value| {
            value.checked_add(
                calibration
                    .cycles_per_processed_block
                    .checked_mul(processed)?,
            )
        })
        .and_then(|value| {
            value.checked_add(
                calibration
                    .cycles_per_ledger_page
                    .checked_mul(additional_pages)?,
            )
        })
        .and_then(|value| {
            value.checked_add(
                calibration
                    .cycles_per_subscriber_notification
                    .checked_mul(coalesced_notification_dispatches)?,
            )
        })
        .ok_or_else(|| anyhow!("projected cycles burn overflow"))?;
    Ok(Batch {
        elapsed,
        polls: 1,
        generated,
        generated_specific_matching,
        processed,
        processed_specific_matching,
        global_notifications,
        specific_notifications,
        coalesced_notification_dispatches,
        maximum_pre_poll: available,
        maximum_post_poll: available - processed,
        burn,
        block_remainder,
        specific_matching_remainder,
        specific_subscriber_remainder,
        overlap_notification_remainder,
    })
}

fn apply_batch(state: &mut SimState, batch: Batch) {
    state.seconds_in_modes[rank(state.mode)] =
        state.seconds_in_modes[rank(state.mode)].saturating_add(batch.elapsed);
    state.now = state.now.saturating_add(batch.elapsed);
    state.backlog = state
        .backlog
        .saturating_add(batch.generated)
        .saturating_sub(batch.processed);
    state.specific_matching_backlog = state
        .specific_matching_backlog
        .saturating_add(batch.generated_specific_matching)
        .saturating_sub(batch.processed_specific_matching);
    state.block_milli_remainder = batch.block_remainder;
    state.specific_matching_basis_remainder = batch.specific_matching_remainder;
    state.specific_subscriber_milli_remainder = batch.specific_subscriber_remainder;
    state.overlap_notification_remainder = batch.overlap_notification_remainder;
    state.maximum_backlog = state
        .maximum_backlog
        .max(state.backlog)
        .max(batch.maximum_pre_poll);
    state.maximum_pre_poll = state.maximum_pre_poll.max(batch.maximum_pre_poll);
    state.maximum_post_poll = state.maximum_post_poll.max(batch.maximum_post_poll);
    state.global_notifications = state
        .global_notifications
        .saturating_add(batch.global_notifications);
    state.specific_notifications = state
        .specific_notifications
        .saturating_add(batch.specific_notifications);
    state.coalesced_notification_dispatches = state
        .coalesced_notification_dispatches
        .saturating_add(batch.coalesced_notification_dispatches);
    state.ordinary_polling_spent = state.ordinary_polling_spent.saturating_add(batch.burn);
    apply_burn(state, batch.burn);
    let _ = batch.polls;
}

fn apply_burn(state: &mut SimState, burn: u128) {
    state.spent = state.spent.saturating_add(burn);
    if burn > state.balance {
        state.balance = 0;
        state.exhausted = true;
    } else {
        state.balance -= burn;
    }
    state.exhausted |= state.balance == 0;
    state.minimum_balance = state.minimum_balance.min(state.balance);
}

fn accrue_without_poll(state: &mut SimState, workload: &Workload, elapsed: u64) {
    state.seconds_in_modes[rank(state.mode)] =
        state.seconds_in_modes[rank(state.mode)].saturating_add(elapsed);
    let numerator = state.block_milli_remainder.saturating_add(
        u128::from(workload.blocks_per_second_milli).saturating_mul(u128::from(elapsed)),
    );
    let generated = numerator / 1_000;
    state.block_milli_remainder = numerator % 1_000;
    let specific_matching = state.specific_matching_basis_remainder.saturating_add(
        generated.saturating_mul(u128::from(workload.specific_matching_blocks_basis_points)),
    );
    let generated_specific_matching = specific_matching / 10_000;
    state.specific_matching_basis_remainder = specific_matching % 10_000;
    if state.mode == Mode::ReserveProtection {
        state.reserve_generated = state.reserve_generated.saturating_add(generated);
    }
    state.backlog = state.backlog.saturating_add(generated);
    state.specific_matching_backlog = state
        .specific_matching_backlog
        .saturating_add(generated_specific_matching);
    state.maximum_backlog = state.maximum_backlog.max(state.backlog);
}

fn change_mode(state: &mut SimState, policy: &Policy, continuous_interval_seconds: u64) {
    let previous = state.mode;
    let next = next_mode(&policy.thresholds, previous, state.balance);
    if next == previous {
        return;
    }
    state.transitions.push(state.now);
    if rank(next) < rank(previous) && state.first_downgrade.is_none() {
        state.first_downgrade = Some(state.now);
        state.downgrade_backlog = Some(state.backlog);
    }
    if next == Mode::ReserveProtection && state.first_reserve.is_none() {
        state.first_reserve = Some(state.now);
        state.reserve_backlog = Some(state.backlog);
    }
    state.mode = next;
    state.next_poll = next
        .delay_seconds(continuous_interval_seconds)
        .map(|delay| state.now.saturating_add(delay));
}

fn funding_schedule(
    profile: &str,
    end: u64,
    assumptions: &FundingAssumptions,
    workload: &Workload,
    seed: u64,
) -> Result<Vec<(u64, u128)>> {
    let yield_basis_points = match profile {
        "zero_effective_yield" | "no_incoming_funding" => 0,
        "low_yield_2_8_percent" => 280,
        _ => assumptions.effective_payout_apy_basis_points,
    };
    if yield_basis_points == 0 {
        return Ok(Vec::new());
    }
    let annual_payout_e8s = annual_payout_e8s(assumptions, workload, yield_basis_points)?;
    let start = assumptions
        .initial_recognition_delay_seconds
        .checked_add(assumptions.conversion_delay_seconds)
        .ok_or_else(|| anyhow!("recognition delay overflow"))?;
    let cadence_days = match profile {
        "regular_daily" => 1,
        "regular_monthly" => 30,
        _ => 7,
    };
    let mut base = regular_schedule(start, end, cadence_days, annual_payout_e8s)?;
    let payouts = match profile {
        "no_incoming_funding" | "zero_effective_yield" => Vec::new(),
        "regular_daily" | "regular_weekly" | "regular_monthly" | "low_yield_2_8_percent" => base,
        "irregular" => {
            let mut state = seed.max(1);
            let mut time = start;
            let mut out = Vec::new();
            while time <= end {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let days = 3 + (state % 16);
                let amount = annual_payout_e8s
                    .checked_mul(u128::from(days))
                    .ok_or_else(|| anyhow!("irregular payout overflow"))?
                    / YEAR_DAYS;
                time = time
                    .checked_add(
                        days.checked_mul(DAY)
                            .ok_or_else(|| anyhow!("payout time overflow"))?,
                    )
                    .ok_or_else(|| anyhow!("payout time overflow"))?;
                if time <= end {
                    out.push((time, amount));
                }
            }
            out
        }
        "seven_day_interruption" => interrupt(&base, 30 * DAY, 7 * DAY),
        "thirty_day_interruption" => interrupt(&base, 30 * DAY, 30 * DAY),
        "ninety_day_interruption" => interrupt(&base, 30 * DAY, 90 * DAY),
        "delayed_payout_and_recovery" => {
            let release = start.saturating_add(90 * DAY);
            let mut held = 0_u128;
            base.retain(|(time, amount)| {
                if *time < release {
                    held = held.saturating_add(*amount);
                    false
                } else {
                    true
                }
            });
            if release <= end && held > 0 {
                base.push((release, held));
                base.sort_unstable_by_key(|item| item.0);
            }
            base
        }
        other => bail!("unknown funding profile `{other}`"),
    };
    payouts
        .into_iter()
        .map(|(time, payout_e8s)| Ok((time, payout_cycles(assumptions, payout_e8s)?)))
        .collect()
}

// Hypothetical continuing tier: no adaptive downgrade or startup backlog.
// Annual income is the mature payout rate, net of one conversion fee per
// cadence, annualized with integer rounding. Recognition delay affects only
// finite-window receipts, never productive principal or the mature rate.
fn fixed_tier_economics_for(
    workload: &Workload,
    calibration: &CostCalibration,
    assumptions: &FundingAssumptions,
    mode: Mode,
    interval: u64,
    payout_cadence_days: u64,
) -> Result<FixedTierEconomics> {
    if interval == 0 || payout_cadence_days == 0 {
        bail!("fixed-tier intervals must be positive");
    }
    let annual = annual_payout_e8s(
        assumptions,
        workload,
        assumptions.effective_payout_apy_basis_points,
    )?;
    let per_payout = annual
        .checked_mul(u128::from(payout_cadence_days))
        .ok_or_else(|| anyhow!("annual payout overflow"))?
        / YEAR_DAYS;
    let income = payout_cycles(assumptions, per_payout)?
        .checked_mul(YEAR_DAYS)
        .ok_or_else(|| anyhow!("annual income overflow"))?
        / u128::from(payout_cadence_days);
    let mut continuing = workload.clone();
    continuing.initial_backlog_blocks = 0;
    let state = SimState {
        now: 0,
        balance: u128::MAX,
        minimum_balance: u128::MAX,
        mode,
        next_poll: Some(interval),
        next_funding_maintenance: FUNDING_MAINTENANCE_SECONDS,
        next_pricing_maintenance: PRICING_MAINTENANCE_SECONDS,
        backlog: 0,
        specific_matching_backlog: 0,
        block_milli_remainder: 0,
        specific_matching_basis_remainder: 0,
        specific_subscriber_milli_remainder: 0,
        overlap_notification_remainder: 0,
        spent: 0,
        ordinary_polling_spent: 0,
        maintenance_spent: 0,
        funded: 0,
        funding_maintenance_runs: 0,
        pricing_maintenance_runs: 0,
        global_notifications: 0,
        specific_notifications: 0,
        coalesced_notification_dispatches: 0,
        seconds_in_modes: [0; 6],
        transitions: Vec::new(),
        first_downgrade: None,
        first_reserve: None,
        maximum_backlog: 0,
        maximum_pre_poll: 0,
        maximum_post_poll: 0,
        downgrade_backlog: None,
        reserve_backlog: None,
        reserve_generated: 0,
        exhausted: false,
    };
    let year_seconds = 365 * DAY;
    let polls = year_seconds / interval;
    let batch = project_batch(&state, &continuing, calibration, interval, polls, interval)?;
    let polling = batch.burn;
    let maintenance = calibration
        .funding_maintenance_cycles
        .unwrap_or_default()
        .checked_mul(u128::from(year_seconds / FUNDING_MAINTENANCE_SECONDS))
        .and_then(|value| {
            value.checked_add(
                calibration
                    .pricing_maintenance_cycles
                    .unwrap_or_default()
                    .checked_mul(u128::from(year_seconds / PRICING_MAINTENANCE_SECONDS))?,
            )
        })
        .ok_or_else(|| anyhow!("annual maintenance overflow"))?;
    let expenditure = polling
        .checked_add(maintenance)
        .ok_or_else(|| anyhow!("annual expenditure overflow"))?;
    let margin = i128::try_from(income)?
        .checked_sub(i128::try_from(expenditure)?)
        .ok_or_else(|| anyhow!("annual margin overflow"))?;
    Ok(FixedTierEconomics {
        workload_id: workload.id.clone(),
        mode,
        effective_poll_interval_seconds: interval,
        payout_cadence_days,
        annual_generated_blocks: batch.generated,
        annual_processed_blocks: batch.processed,
        mature_annual_income_cycles: income,
        annualized_operating_expenditure_cycles: expenditure,
        steady_state_annual_margin_cycles: margin,
    })
}

fn annual_payout_e8s(
    assumptions: &FundingAssumptions,
    workload: &Workload,
    yield_basis_points: u64,
) -> Result<u128> {
    let backing = &workload.productive_endowment_backing;
    let account_price = rounded_admission_price(
        assumptions.account_reference_endowment_icp_e8s,
        backing.admission_price_basis_points_of_reference,
    )?;
    let range_price = rounded_admission_price(
        assumptions.range_reference_endowment_icp_e8s,
        backing.admission_price_basis_points_of_reference,
    )?;
    let global_price = rounded_admission_price(
        assumptions.global_reference_endowment_icp_e8s,
        backing.admission_price_basis_points_of_reference,
    )?;
    let subscription_principal = u128::from(backing.account_endowments)
        .checked_mul(account_price)
        .and_then(|value| {
            value.checked_add(u128::from(backing.range_endowments).checked_mul(range_price)?)
        })
        .and_then(|value| {
            value.checked_add(u128::from(backing.global_endowments).checked_mul(global_price)?)
        })
        .ok_or_else(|| anyhow!("subscription endowment overflow"))?;
    let endowed_icp_e8s = backing
        .protocol_endowment_icp_e8s
        .checked_add(subscription_principal)
        .ok_or_else(|| anyhow!("productive endowment overflow"))?;
    let payout_e8s = endowed_icp_e8s
        .checked_mul(u128::from(yield_basis_points))
        .ok_or_else(|| anyhow!("yield calculation overflow"))?
        / 10_000;
    let after_shortfall = payout_e8s
        .checked_mul(u128::from(
            10_000_u64.saturating_sub(assumptions.delivery_shortfall_basis_points),
        ))
        .ok_or_else(|| anyhow!("payout shortfall overflow"))?
        / 10_000;
    Ok(after_shortfall
        .checked_mul(u128::from(
            10_000_u64.saturating_sub(assumptions.surplus_diversion_basis_points),
        ))
        .ok_or_else(|| anyhow!("payout diversion overflow"))?
        / 10_000)
}

fn rounded_admission_price(reference_e8s: u128, basis_points: u64) -> Result<u128> {
    if !(1..=10_000).contains(&basis_points) || !reference_e8s.is_multiple_of(ICP_E8S) {
        bail!("invalid admission price multiplier or non-whole-ICP reference")
    }
    (reference_e8s / ICP_E8S)
        .checked_mul(u128::from(basis_points))
        .and_then(|value| value.checked_add(9_999))
        .and_then(|value| (value / 10_000).checked_mul(ICP_E8S))
        .ok_or_else(|| anyhow!("dynamic admission price overflow"))
}

fn payout_cycles(assumptions: &FundingAssumptions, payout_e8s: u128) -> Result<u128> {
    payout_e8s
        .saturating_sub(u128::from(assumptions.conversion_fee_e8s))
        .checked_mul(u128::from(assumptions.xdr_per_icp_permyriad))
        .and_then(|value| value.checked_mul(TRILLION))
        .map(|value| value / ICP_E8S / 10_000)
        .ok_or_else(|| anyhow!("ICP/XDR conversion overflow"))
}

fn regular_schedule(
    start: u64,
    end: u64,
    cadence_days: u64,
    annual: u128,
) -> Result<Vec<(u64, u128)>> {
    let interval = cadence_days
        .checked_mul(DAY)
        .ok_or_else(|| anyhow!("payout interval overflow"))?;
    if interval == 0 {
        bail!("payout interval must be positive");
    }
    let amount = annual
        .checked_mul(u128::from(cadence_days))
        .ok_or_else(|| anyhow!("payout overflow"))?
        / YEAR_DAYS;
    let mut out = Vec::new();
    // Pay completed accrual periods, rather than crediting a period in advance.
    let mut time = start
        .checked_add(interval)
        .ok_or_else(|| anyhow!("payout time overflow"))?;
    while time <= end {
        out.push((time, amount));
        time = time
            .checked_add(interval)
            .ok_or_else(|| anyhow!("payout time overflow"))?;
    }
    Ok(out)
}

fn interrupt(schedule: &[(u64, u128)], start: u64, duration: u64) -> Vec<(u64, u128)> {
    let end = start.saturating_add(duration);
    let mut held = 0_u128;
    let mut out = Vec::new();
    for &(time, amount) in schedule {
        if (start..end).contains(&time) {
            held = held.saturating_add(amount);
        } else {
            out.push((time, amount));
        }
    }
    if held > 0 {
        out.push((end, held));
        out.sort_unstable_by_key(|item| item.0);
    }
    out
}

pub fn default_model_config() -> ModelConfig {
    ModelConfig {
        funding_assumptions: FundingAssumptions {
            account_reference_endowment_icp_e8s: 10 * ICP_E8S,
            range_reference_endowment_icp_e8s: 20 * ICP_E8S,
            global_reference_endowment_icp_e8s: 1_000 * ICP_E8S,
            gross_maturity_apy_basis_points: 700,
            effective_payout_apy_basis_points: 560,
            xdr_per_icp_permyriad: 20_000,
            initial_recognition_delay_seconds: 14 * DAY,
            conversion_delay_seconds: 3_600,
            conversion_fee_e8s: 10_000,
            delivery_shortfall_basis_points: 0,
            surplus_diversion_basis_points: 0,
        },
        policies: vec![
            Policy {
                id: "current".into(),
                thresholds: Thresholds::current(),
            },
            Policy {
                id: "candidate_125T_continuous_entry".into(),
                thresholds: Thresholds::candidate_125t(),
            },
            Policy {
                id: "conservative".into(),
                thresholds: Thresholds::conservative(),
            },
            Policy {
                id: "performance_oriented".into(),
                thresholds: Thresholds::performance(),
            },
        ],
        workloads: vec![
            Workload {
                id: "baseline".into(),
                architecture: "canonical_icp_shared_scan".into(),
                blocks_per_second_milli: 0,
                specific_matching_blocks_basis_points: 0,
                specific_subscribers_per_matching_block_milli: 0,
                global_subscribers: 0,
                specific_subscribers: 0,
                global_specific_overlap_subscribers: 0,
                initial_backlog_blocks: 0,
                assumed_blocks_per_poll_capacity: 4_096,
                productive_endowment_backing: ProductiveEndowmentBacking {
                    protocol_endowment_icp_e8s: 3_000 * ICP_E8S,
                    account_endowments: 0,
                    range_endowments: 0,
                    global_endowments: 0,
                    admission_price_basis_points_of_reference: 10_000,
                    coupled_to_subscriber_workload: true,
                },
            },
            Workload {
                id: "expected".into(),
                architecture: "canonical_icp_shared_scan".into(),
                blocks_per_second_milli: 1_000,
                specific_matching_blocks_basis_points: 500,
                specific_subscribers_per_matching_block_milli: 1_000,
                global_subscribers: 1,
                specific_subscribers: 5,
                global_specific_overlap_subscribers: 0,
                initial_backlog_blocks: 0,
                assumed_blocks_per_poll_capacity: 4_096,
                productive_endowment_backing: ProductiveEndowmentBacking {
                    protocol_endowment_icp_e8s: 3_000 * ICP_E8S,
                    account_endowments: 3,
                    range_endowments: 2,
                    global_endowments: 1,
                    admission_price_basis_points_of_reference: 10_000,
                    coupled_to_subscriber_workload: true,
                },
            },
            Workload {
                id: "expected_generic".into(),
                architecture: "generic_icrc3_separate_admission_scan".into(),
                blocks_per_second_milli: 1_000,
                specific_matching_blocks_basis_points: 500,
                specific_subscribers_per_matching_block_milli: 1_000,
                global_subscribers: 1,
                specific_subscribers: 5,
                global_specific_overlap_subscribers: 0,
                initial_backlog_blocks: 0,
                assumed_blocks_per_poll_capacity: 4_096,
                productive_endowment_backing: ProductiveEndowmentBacking {
                    protocol_endowment_icp_e8s: 0,
                    account_endowments: 3,
                    range_endowments: 2,
                    global_endowments: 1,
                    admission_price_basis_points_of_reference: 10_000,
                    coupled_to_subscriber_workload: true,
                },
            },
            Workload {
                id: "high_activity".into(),
                architecture: "canonical_icp_shared_scan".into(),
                blocks_per_second_milli: 5_000,
                specific_matching_blocks_basis_points: 1_000,
                specific_subscribers_per_matching_block_milli: 1_000,
                global_subscribers: 5,
                specific_subscribers: 25,
                global_specific_overlap_subscribers: 5,
                initial_backlog_blocks: 1_024,
                assumed_blocks_per_poll_capacity: 4_096,
                productive_endowment_backing: ProductiveEndowmentBacking {
                    protocol_endowment_icp_e8s: 3_000 * ICP_E8S,
                    account_endowments: 20,
                    range_endowments: 5,
                    global_endowments: 5,
                    admission_price_basis_points_of_reference: 10_000,
                    coupled_to_subscriber_workload: true,
                },
            },
            Workload {
                id: "stress".into(),
                architecture: "canonical_icp_shared_scan".into(),
                blocks_per_second_milli: 20_000,
                specific_matching_blocks_basis_points: 2_500,
                specific_subscribers_per_matching_block_milli: 2_000,
                global_subscribers: 100,
                specific_subscribers: 100,
                global_specific_overlap_subscribers: 50,
                initial_backlog_blocks: 4_096,
                assumed_blocks_per_poll_capacity: 4_096,
                productive_endowment_backing: ProductiveEndowmentBacking {
                    protocol_endowment_icp_e8s: 3_000 * ICP_E8S,
                    account_endowments: 0,
                    range_endowments: 0,
                    global_endowments: 0,
                    admission_price_basis_points_of_reference: 10_000,
                    coupled_to_subscriber_workload: false,
                },
            },
        ],
        continuous_effective_poll_interval_seconds: 1,
        continuous_effective_poll_intervals_seconds: vec![1, 2, 5, 10],
        equal_starting_balances_cycles: vec![
            75 * TRILLION,
            100 * TRILLION,
            125 * TRILLION,
            160 * TRILLION,
        ],
        continuous_runway_targets_days: vec![7, 14, 30, 60, 90],
        lower_tier_runway_targets_days: [30, 30, 30, 30],
    }
}

pub fn read_model_config(path: &Path) -> Result<ModelConfig> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let config: ModelConfig =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    validate_model_config(&config)?;
    Ok(config)
}

fn validate_model_config(config: &ModelConfig) -> Result<()> {
    if config.continuous_effective_poll_interval_seconds == 0 {
        bail!("continuous_effective_poll_interval_seconds must be positive")
    }
    if config
        .continuous_effective_poll_intervals_seconds
        .is_empty()
        || config
            .continuous_effective_poll_intervals_seconds
            .contains(&0)
        || !config
            .continuous_effective_poll_intervals_seconds
            .contains(&config.continuous_effective_poll_interval_seconds)
    {
        bail!("Continuous interval sensitivity must be nonempty, positive, and include the reference interval")
    }
    if config.equal_starting_balances_cycles.is_empty()
        || config.equal_starting_balances_cycles.contains(&0)
    {
        bail!("equal starting balances must be nonempty and positive")
    }
    if config.continuous_runway_targets_days.is_empty()
        || config.continuous_runway_targets_days.contains(&0)
        || config.lower_tier_runway_targets_days.contains(&0)
    {
        bail!("runway objectives must be nonempty and positive")
    }
    if config.policies.is_empty() || config.workloads.is_empty() {
        bail!("model config must contain policies and workloads")
    }
    if config.funding_assumptions.delivery_shortfall_basis_points > 10_000
        || config.funding_assumptions.surplus_diversion_basis_points > 10_000
        || config.funding_assumptions.xdr_per_icp_permyriad == 0
        || config
            .funding_assumptions
            .account_reference_endowment_icp_e8s
            == 0
        || config.funding_assumptions.range_reference_endowment_icp_e8s == 0
        || config
            .funding_assumptions
            .global_reference_endowment_icp_e8s
            == 0
    {
        bail!("funding percentage assumptions must be in range and ICP/XDR must be positive")
    }
    if !config.policies.iter().any(|policy| policy.id == "current")
        || !config
            .workloads
            .iter()
            .any(|workload| workload.id == "expected")
    {
        bail!("model config requires `current` policy and `expected` workload")
    }
    for policy in &config.policies {
        policy
            .thresholds
            .validate()
            .map_err(|error| anyhow!("policy {}: {error}", policy.id))?;
    }
    for workload in &config.workloads {
        if workload.specific_matching_blocks_basis_points > 10_000 {
            bail!(
                "workload {} has specific matching traffic above 100%",
                workload.id
            )
        }
        if !(1..=10_000).contains(
            &workload
                .productive_endowment_backing
                .admission_price_basis_points_of_reference,
        ) {
            bail!(
                "workload {} has an admission-price multiplier outside 1..=10000",
                workload.id
            )
        }
        if workload.assumed_blocks_per_poll_capacity == 0 {
            bail!("workload {} has zero assumed poll capacity", workload.id)
        }
        if workload.specific_subscribers == 0
            && workload.specific_subscribers_per_matching_block_milli != 0
        {
            bail!(
                "workload {} models specific matches without specific subscribers",
                workload.id
            )
        }
        if workload.global_specific_overlap_subscribers > workload.global_subscribers
            || workload.global_specific_overlap_subscribers > workload.specific_subscribers
        {
            bail!(
                "workload {} has overlap larger than a subscriber population",
                workload.id
            )
        }
        if workload
            .productive_endowment_backing
            .coupled_to_subscriber_workload
        {
            let backing = &workload.productive_endowment_backing;
            if backing.global_endowments < workload.global_subscribers {
                bail!("workload {} has unbacked global subscribers", workload.id)
            }
            if backing
                .account_endowments
                .saturating_add(backing.range_endowments)
                < workload.specific_subscribers
            {
                bail!("workload {} has unbacked specific subscribers", workload.id)
            }
        }
    }
    Ok(())
}

fn max_rolling_transitions(transitions: &[u64], window: u64) -> u64 {
    let mut maximum = 0_usize;
    let mut start = 0_usize;
    for end in 0..transitions.len() {
        while transitions[end].saturating_sub(transitions[start]) > window {
            start += 1;
        }
        maximum = maximum.max(end - start + 1);
    }
    maximum as u64
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[(values.len() - 1) / 2]
}

fn write_threshold_csv(set: &SimulationSet, output: &Path) -> Result<()> {
    let mut csv = String::from("scenario_id,simulation_kind,policy,workload,funding_profile,horizon_days,initial_cycles,initial_mode,continuous_interval_seconds,funding_cycles,expenditure_cycles,ordinary_polling_cycles,maintenance_cycles,funding_maintenance_runs,pricing_maintenance_runs,global_notification_matches,specific_notification_matches,coalesced_notification_dispatches,ending_cycles,minimum_cycles,transitions,max_transitions_7d,first_downgrade_seconds,reserve_protection_seconds,ending_backlog,projected_latency_seconds,liquidity_maintained,window_funding_covers_expenditure,maximum_active_pre_poll_pending,maximum_active_post_poll_outstanding,first_downgrade_outstanding,first_reserve_outstanding,reserve_arrivals,maximum_backlog,funding_surplus,funding_shortfall,reserve_suspended_seconds,exhausted\n");
    for result in &set.results {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{:?},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            result.scenario_id,
            result.simulation_kind,
            result.policy_id,
            result.workload_id,
            result.funding_profile,
            result.horizon_days,
            result.initial_cycles,
            result.initial_mode,
            result.continuous_effective_poll_interval_seconds,
            result.total_funding_cycles,
            result.total_projected_expenditure_cycles,
            result.total_ordinary_polling_cycles,
            result.total_maintenance_cycles,
            result.funding_maintenance_runs,
            result.pricing_maintenance_runs,
            result.global_notifications,
            result.specific_notifications,
            result.coalesced_notification_dispatches,
            result.ending_cycles,
            result.minimum_cycles,
            result.tier_transitions,
            result.maximum_transitions_in_rolling_seven_days,
            result
                .first_downgrade_seconds
                .map_or(String::new(), |v| v.to_string()),
            result
                .reserve_protection_seconds
                .map_or(String::new(), |v| v.to_string()),
            result.ending_backlog_blocks,
            result.projected_latency_seconds,
            result.liquidity_maintained,
            result.window_funding_covers_expenditure,
            result.maximum_active_pre_poll_pending_blocks,
            result.maximum_active_post_poll_outstanding_blocks,
            result.outstanding_blocks_at_first_downgrade.map_or(String::new(), |v| v.to_string()),
            result.outstanding_blocks_at_first_reserve.map_or(String::new(), |v| v.to_string()),
            result.additional_reserve_backlog_blocks,
            result.maximum_backlog_blocks,
            result.funding_surplus_cycles,
            result.funding_shortfall_cycles,
            result.seconds_in_modes[0],
            result.exhausted,
        ));
    }
    fs::write(output.join("threshold-comparison.csv"), csv)?;
    Ok(())
}

pub fn read_simulations(path: &Path) -> Result<SimulationSet> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    #[derive(serde::Deserialize)]
    struct SchemaVersion {
        schema_version: u32,
    }
    let version: SchemaVersion =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    if version.schema_version != 3 {
        bail!("unsupported simulation schema; regenerate schema 3 outputs from unchanged raw measurements");
    }
    let set: SimulationSet =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    if set.schema_version != 3 || set.results.is_empty() {
        bail!("invalid or empty simulation dataset")
    }
    Ok(set)
}

pub fn default_output_for(input: &Path) -> PathBuf {
    input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration() -> CostCalibration {
        CostCalibration {
            architecture: "test".into(),
            empty_poll_cycles: 1_000_000,
            cycles_per_processed_block: 100_000,
            cycles_per_ledger_page: 500_000,
            cycles_per_subscriber_notification: 5_000_000,
            funding_maintenance_cycles: Some(20_000),
            pricing_maintenance_cycles: Some(30_000),
            largest_completed_backlog: 4_096,
            sources: vec!["fixture".into()],
        }
    }

    fn policy() -> Policy {
        Policy {
            id: "current".into(),
            thresholds: Thresholds::current(),
        }
    }

    fn assumptions() -> FundingAssumptions {
        default_model_config().funding_assumptions
    }

    fn workloads() -> Vec<Workload> {
        default_model_config().workloads
    }

    fn test_workload(
        blocks_per_second_milli: u64,
        matching_basis_points: u64,
        global_subscribers: u64,
        specific_subscribers: u64,
        subscribers_per_match_milli: u64,
    ) -> Workload {
        Workload {
            id: "test".into(),
            architecture: "test".into(),
            blocks_per_second_milli,
            specific_matching_blocks_basis_points: matching_basis_points,
            specific_subscribers_per_matching_block_milli: subscribers_per_match_milli,
            global_subscribers,
            specific_subscribers,
            global_specific_overlap_subscribers: 0,
            initial_backlog_blocks: 0,
            assumed_blocks_per_poll_capacity: 4_096,
            productive_endowment_backing: ProductiveEndowmentBacking {
                protocol_endowment_icp_e8s: 0,
                account_endowments: 0,
                range_endowments: 0,
                global_endowments: 0,
                admission_price_basis_points_of_reference: 10_000,
                coupled_to_subscriber_workload: false,
            },
        }
    }

    fn test_state() -> SimState {
        SimState {
            now: 0,
            balance: 100 * TRILLION,
            minimum_balance: 100 * TRILLION,
            mode: Mode::Continuous,
            next_poll: Some(1),
            next_funding_maintenance: FUNDING_MAINTENANCE_SECONDS,
            next_pricing_maintenance: PRICING_MAINTENANCE_SECONDS,
            backlog: 0,
            specific_matching_backlog: 0,
            block_milli_remainder: 0,
            specific_matching_basis_remainder: 0,
            specific_subscriber_milli_remainder: 0,
            overlap_notification_remainder: 0,
            spent: 0,
            ordinary_polling_spent: 0,
            maintenance_spent: 0,
            funded: 0,
            funding_maintenance_runs: 0,
            pricing_maintenance_runs: 0,
            global_notifications: 0,
            specific_notifications: 0,
            coalesced_notification_dispatches: 0,
            seconds_in_modes: [0; 6],
            transitions: vec![],
            first_downgrade: None,
            first_reserve: None,
            maximum_backlog: 0,
            maximum_pre_poll: 0,
            maximum_post_poll: 0,
            downgrade_backlog: None,
            reserve_backlog: None,
            reserve_generated: 0,
            exhausted: false,
        }
    }

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../observations/cadence")
                .join(format!(
                    "regression-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn model_and_report_preserve_measurements_in_place_and_separate_outputs() {
        use sha2::{Digest, Sha256};
        let directory = TestDirectory::new();
        let config_path = directory.0.join("config.json");
        let mut config = default_model_config();
        config.policies.retain(|policy| policy.id == "current");
        config
            .workloads
            .retain(|workload| workload.id == "expected");
        config.workloads[0].blocks_per_second_milli = 0;
        fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        let input = directory.0.join("measurements.json");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/example-measurements.json"),
            &input,
        )
        .unwrap();
        let csv = directory.0.join("measurements.csv");
        fs::write(&csv, b"original CSV\n").unwrap();
        let hashes = || {
            [
                Sha256::digest(fs::read(&input).unwrap()),
                Sha256::digest(fs::read(&csv).unwrap()),
            ]
        };
        let before = hashes();
        let metadata = [
            fs::metadata(&input).unwrap().modified().unwrap(),
            fs::metadata(&csv).unwrap().modified().unwrap(),
        ];
        fs::create_dir(directory.0.join("child")).unwrap();
        let absolute = fs::canonicalize(&directory.0).unwrap();
        let relative = directory
            .0
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap()
            .join("child/../.");
        let first = run(&input, &directory.0, 77, Some(&config_path)).unwrap();
        let report = crate::report::run(&directory.0).unwrap();
        assert_eq!(
            first,
            run(&input, &directory.0, 77, Some(&config_path)).unwrap()
        );
        assert_eq!(report, crate::report::run(&directory.0).unwrap());
        for output in [&absolute, &relative] {
            copy_measurement(&input, &output.join("measurements.json")).unwrap();
            copy_measurement(&csv, &output.join("measurements.csv")).unwrap();
            assert_eq!(before, hashes());
        }
        #[cfg(unix)]
        {
            let alias = directory.0.join("alias");
            std::os::unix::fs::symlink(&absolute, &alias).unwrap();
            copy_measurement(&input, &alias.join("measurements.json")).unwrap();
            copy_measurement(&csv, &alias.join("measurements.csv")).unwrap();
            let separate = directory.0.join("symlink-destination");
            fs::create_dir(&separate).unwrap();
            std::os::unix::fs::symlink(&input, separate.join("measurements.json")).unwrap();
            std::os::unix::fs::symlink(&csv, separate.join("measurements.csv")).unwrap();
            copy_measurement(&input, &separate.join("measurements.json")).unwrap();
            copy_measurement(&csv, &separate.join("measurements.csv")).unwrap();
            let hard = directory.0.join("hard.json");
            fs::hard_link(&input, &hard).unwrap();
            copy_measurement(&input, &hard).unwrap();
        }
        let separate = directory.0.join("separate");
        run(&input, &separate, 77, Some(&config_path)).unwrap();
        crate::report::run(&separate).unwrap();
        for name in [
            "measurements.json",
            "measurements.csv",
            "simulations.json",
            "threshold-comparison.csv",
            "report.md",
        ] {
            assert!(separate.join(name).is_file());
        }
        assert_eq!(
            fs::read(&input).unwrap(),
            fs::read(separate.join("measurements.json")).unwrap()
        );
        assert_eq!(
            fs::read(&csv).unwrap(),
            fs::read(separate.join("measurements.csv")).unwrap()
        );
        assert_eq!(before, hashes());
        assert_eq!(
            metadata,
            [
                fs::metadata(&input).unwrap().modified().unwrap(),
                fs::metadata(&csv).unwrap().modified().unwrap()
            ]
        );
        let failure = directory.0.join("failure");
        fs::create_dir_all(failure.join("measurements.csv")).unwrap();
        let error = run(&input, &failure, 77, Some(&config_path)).unwrap_err();
        assert!(format!("{error:#}").contains("copy"));
        assert_eq!(before, hashes());
    }

    #[test]
    fn active_queue_peaks_and_growing_lag_match_reference() {
        for interval in [1, 10, 120, 600, 3_600] {
            for initial in [0, 1_000, 10_000] {
                for arrival in [0, 1_000, 2_000] {
                    let mut state = test_state();
                    state.backlog = initial;
                    let mut workload = test_workload(arrival, 0, 1, 0, 0);
                    if interval == 3_600 && arrival == 1_000 && initial == 0 {
                        workload.assumed_blocks_per_poll_capacity = 3_600;
                    }
                    let batch =
                        project_batch(&state, &workload, &calibration(), interval, 20, interval)
                            .unwrap();
                    let mut pending = initial;
                    let mut pre = 0;
                    let mut post = 0;
                    for _ in 0..20 {
                        pending += u128::from(arrival) * u128::from(interval) / 1_000;
                        pre = pre.max(pending);
                        pending = pending
                            .saturating_sub(u128::from(workload.assumed_blocks_per_poll_capacity));
                        post = post.max(pending);
                    }
                    assert_eq!(batch.maximum_pre_poll, pre);
                    assert_eq!(batch.maximum_post_poll, post);
                    apply_batch(&mut state, batch);
                    assert_eq!(state.backlog, pending);
                    assert_eq!(state.now, 20 * interval);
                    if initial == 0 && arrival == 1_000 {
                        assert_eq!(post, 0);
                        assert_eq!(pre, u128::from(interval));
                    }
                }
            }
        }
    }

    #[test]
    fn downgrade_reserve_and_top_up_capture_queue_and_timing() {
        let mut state = test_state();
        let workload = test_workload(1_000, 0, 0, 0, 0);
        state.now = 100;
        state.backlog = 123;
        state.balance = 59 * TRILLION;
        change_mode(&mut state, &policy(), 1);
        assert_eq!(state.mode, Mode::VeryFast);
        assert_eq!(state.next_poll, Some(110));
        assert_eq!(state.first_downgrade, Some(100));
        assert_eq!(state.downgrade_backlog, Some(123));
        state.balance = TRILLION - 1;
        change_mode(&mut state, &policy(), 1);
        assert_eq!(state.first_reserve, Some(100));
        assert_eq!(state.reserve_backlog, Some(123));
        assert_eq!(state.next_poll, None);
        accrue_without_poll(&mut state, &workload, 3_600);
        state.now += 3_600;
        assert_eq!(state.backlog, 3_723);
        assert_eq!(state.reserve_generated, 3_600);
        assert_eq!(state.seconds_in_modes[0], 3_600);
        state.balance = 100 * TRILLION;
        change_mode(&mut state, &policy(), 1);
        assert_eq!(state.next_poll, Some(3_701));
        let batch = project_batch(&state, &workload, &calibration(), 3_701, 1, 1).unwrap();
        apply_batch(&mut state, batch);
        assert_eq!(state.backlog, 0);
        assert_eq!(state.reserve_generated, 3_600);
        assert_eq!(state.reserve_backlog, Some(123));
    }

    #[test]
    fn adaptive_batch_transitions_match_bounded_poll_reference() {
        let assumptions = assumptions();
        let mut calibration = calibration();
        calibration.empty_poll_cycles = TRILLION;
        calibration.funding_maintenance_cycles = Some(0);
        calibration.pricing_maintenance_cycles = Some(0);
        let mut workload = test_workload(1_000, 0, 1, 0, 0);
        workload.initial_backlog_blocks = 10_000;
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 1,
        };
        let projected = simulate(
            &policy(),
            &workload,
            &context,
            "reference",
            "no_incoming_funding",
            1,
            100 * TRILLION,
        )
        .unwrap();
        let mut reference = test_state();
        reference.backlog = 10_000;
        reference.maximum_backlog = 10_000;
        while let Some(time) = reference.next_poll.filter(|time| *time <= DAY) {
            let batch = project_single_poll(&reference, &workload, &calibration, time).unwrap();
            apply_batch(&mut reference, batch);
            reference.next_poll = reference
                .mode
                .delay_seconds(1)
                .map(|delay| reference.now + delay);
            change_mode(&mut reference, &policy(), 1);
        }
        let elapsed = DAY - reference.now;
        accrue_without_poll(&mut reference, &workload, elapsed);
        assert_eq!(projected.first_downgrade_seconds, reference.first_downgrade);
        assert_eq!(
            projected.reserve_protection_seconds,
            reference.first_reserve
        );
        assert_eq!(
            projected.outstanding_blocks_at_first_downgrade,
            reference.downgrade_backlog
        );
        assert_eq!(
            projected.outstanding_blocks_at_first_reserve,
            reference.reserve_backlog
        );
        assert_eq!(projected.ending_backlog_blocks, reference.backlog);
        assert_eq!(
            projected.maximum_active_pre_poll_pending_blocks,
            reference.maximum_pre_poll
        );
        assert_eq!(
            projected.maximum_active_post_poll_outstanding_blocks,
            reference.maximum_post_poll
        );
        assert_eq!(
            projected.additional_reserve_backlog_blocks,
            reference.reserve_generated
        );
        assert_eq!(projected.seconds_in_modes, reference.seconds_in_modes);
        assert_eq!(
            projected.total_projected_expenditure_cycles,
            reference.spent
        );
    }

    #[test]
    fn identical_receipts_on_different_schedules_change_simulated_minimum() {
        let mut assumptions = assumptions();
        assumptions.initial_recognition_delay_seconds = 0;
        assumptions.conversion_delay_seconds = 0;
        assumptions.conversion_fee_e8s = 0;
        assumptions.effective_payout_apy_basis_points = 10_000;
        let mut workload = test_workload(0, 0, 0, 0, 0);
        workload
            .productive_endowment_backing
            .protocol_endowment_icp_e8s = 365 * ICP_E8S;
        let mut calibration = calibration();
        calibration.empty_poll_cycles = 30_000_000;
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 1,
        };
        let daily = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "regular_daily",
            360,
            100 * TRILLION,
        )
        .unwrap();
        let monthly = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "regular_monthly",
            360,
            100 * TRILLION,
        )
        .unwrap();
        assert_eq!(daily.total_funding_cycles, monthly.total_funding_cycles);
        assert_ne!(daily.minimum_cycles, monthly.minimum_cycles);
    }

    #[test]
    fn mature_margin_window_coverage_and_liquidity_are_independent() {
        let mut assumptions = assumptions();
        assumptions.conversion_fee_e8s = 0;
        assumptions.conversion_delay_seconds = 0;
        let mut workload = test_workload(0, 0, 0, 0, 0);
        workload
            .productive_endowment_backing
            .protocol_endowment_icp_e8s = 10_000 * ICP_E8S;
        let mut calibration = calibration();
        calibration.empty_poll_cycles = 30_000_000;
        let economics = fixed_tier_economics_for(
            &workload,
            &calibration,
            &assumptions,
            Mode::Continuous,
            1,
            7,
        )
        .unwrap();
        assert!(economics.steady_state_annual_margin_cycles > 0);
        assumptions.initial_recognition_delay_seconds = 180 * DAY;
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 1,
        };
        let delayed = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "regular_weekly",
            365,
            2_000 * TRILLION,
        )
        .unwrap();
        assert!(!delayed.window_funding_covers_expenditure);
        assert!(delayed.liquidity_maintained);
        assumptions.initial_recognition_delay_seconds = 0;
        assert_eq!(
            economics,
            fixed_tier_economics_for(
                &workload,
                &calibration,
                &assumptions,
                Mode::Continuous,
                1,
                7
            )
            .unwrap()
        );
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 1,
        };
        let immediate = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "regular_weekly",
            365,
            2_000 * TRILLION,
        )
        .unwrap();
        assert!(immediate.window_funding_covers_expenditure);
        assert!(immediate.total_funding_cycles > delayed.total_funding_cycles);
        let low_liquidity = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "regular_monthly",
            365,
            TRILLION / 2,
        )
        .unwrap();
        assert!(low_liquidity.window_funding_covers_expenditure);
        assert!(!low_liquidity.liquidity_maintained);
        assert!(low_liquidity.reserve_protection_seconds.is_some());
        // Equal gross receipts: five daily periods and one five-day period.
        let daily = regular_schedule(0, 5 * DAY, 1, 365 * ICP_E8S).unwrap();
        let spaced = regular_schedule(0, 5 * DAY, 5, 365 * ICP_E8S).unwrap();
        assert_eq!(
            daily.iter().map(|x| x.1).sum::<u128>(),
            spaced.iter().map(|x| x.1).sum::<u128>()
        );
        let minimum = |schedule: Vec<(u64, u128)>| {
            let mut balance = 10 * ICP_E8S;
            let mut minimum = balance;
            let mut previous = 0;
            for (time, amount) in schedule {
                balance -= u128::from(time - previous) * ICP_E8S / DAY as u128;
                minimum = minimum.min(balance);
                balance += amount;
                previous = time;
            }
            minimum
        };
        assert!(minimum(daily) > minimum(spaced));
    }

    #[test]
    fn fixed_tier_economics_respect_fees_yield_conversion_and_affordability() {
        let mut assumptions = assumptions();
        let workload = workloads()[1].clone();
        let calibration = calibration();
        let continuous = fixed_tier_economics_for(
            &workload,
            &calibration,
            &assumptions,
            Mode::Continuous,
            1,
            7,
        )
        .unwrap();
        let economy = fixed_tier_economics_for(
            &workload,
            &calibration,
            &assumptions,
            Mode::Economy,
            3_600,
            7,
        )
        .unwrap();
        assert!(
            economy.annualized_operating_expenditure_cycles
                < continuous.annualized_operating_expenditure_cycles
        );
        assumptions.effective_payout_apy_basis_points = 10;
        let continuous = fixed_tier_economics_for(
            &workload,
            &calibration,
            &assumptions,
            Mode::Continuous,
            1,
            7,
        )
        .unwrap();
        let economy = fixed_tier_economics_for(
            &workload,
            &calibration,
            &assumptions,
            Mode::Economy,
            3_600,
            7,
        )
        .unwrap();
        assert!(continuous.steady_state_annual_margin_cycles < 0);
        assert!(economy.steady_state_annual_margin_cycles > 0);
        let annual = annual_payout_e8s(&assumptions, &workload, 10).unwrap();
        assert_eq!(
            economy.mature_annual_income_cycles,
            payout_cycles(&assumptions, annual * 7 / 365).unwrap() * 365 / 7
        );
        assumptions.effective_payout_apy_basis_points = 0;
        assert_eq!(
            fixed_tier_economics_for(
                &workload,
                &calibration,
                &assumptions,
                Mode::Continuous,
                1,
                7
            )
            .unwrap()
            .mature_annual_income_cycles,
            0
        );
        assumptions.effective_payout_apy_basis_points = 560;
        assumptions.delivery_shortfall_basis_points = 1_000;
        assumptions.surplus_diversion_basis_points = 2_000;
        let reduced = fixed_tier_economics_for(
            &workload,
            &calibration,
            &assumptions,
            Mode::Continuous,
            1,
            1,
        )
        .unwrap();
        assert!(reduced.mature_annual_income_cycles < continuous.mature_annual_income_cycles * 56);
        assert!(payout_cycles(&assumptions, u128::MAX).is_err());
        let mut enormous = workload;
        enormous
            .productive_endowment_backing
            .protocol_endowment_icp_e8s = u128::MAX;
        assert!(annual_payout_e8s(&assumptions, &enormous, 560).is_err());
    }

    #[test]
    fn current_policy_matches_production_for_boundaries_and_current_modes() {
        let policy = Thresholds::current();
        let boundaries = [
            TRILLION,
            2 * TRILLION,
            3 * TRILLION,
            5 * TRILLION,
            6 * TRILLION,
            10 * TRILLION,
            15 * TRILLION,
            25 * TRILLION,
            60 * TRILLION,
            100 * TRILLION,
        ];
        for current in Mode::ALL {
            for boundary in boundaries {
                for balance in [boundary - 1, boundary, boundary + 1] {
                    let production: Mode = event_horizon::next_mode(current.into(), balance).into();
                    assert_eq!(next_mode(&policy, current, balance), production);
                }
            }
        }
    }

    #[test]
    fn large_u128_funding_conversion_does_not_overflow() {
        let assumptions = assumptions();
        let mut workload = workloads()[1].clone();
        workload
            .productive_endowment_backing
            .protocol_endowment_icp_e8s = 1_000_000 * ICP_E8S;
        let payout = annual_payout_e8s(&assumptions, &workload, 10_000).unwrap();
        assert!(payout_cycles(&assumptions, payout).unwrap() > 1_000 * TRILLION);
    }

    #[test]
    fn one_trillion_cycles_is_one_xdr_at_the_model_boundary() {
        let assumptions = FundingAssumptions {
            account_reference_endowment_icp_e8s: 10 * ICP_E8S,
            range_reference_endowment_icp_e8s: 20 * ICP_E8S,
            global_reference_endowment_icp_e8s: 1_000 * ICP_E8S,
            gross_maturity_apy_basis_points: 10_000,
            effective_payout_apy_basis_points: 10_000,
            xdr_per_icp_permyriad: 10_000,
            initial_recognition_delay_seconds: 0,
            conversion_delay_seconds: 0,
            conversion_fee_e8s: 0,
            delivery_shortfall_basis_points: 0,
            surplus_diversion_basis_points: 0,
        };
        assert_eq!(
            payout_cycles(
                &assumptions,
                annual_payout_e8s(
                    &assumptions,
                    &{
                        let mut workload = workloads()[0].clone();
                        workload
                            .productive_endowment_backing
                            .protocol_endowment_icp_e8s = ICP_E8S;
                        workload
                    },
                    10_000
                )
                .unwrap(),
            )
            .unwrap(),
            TRILLION
        );
    }

    #[test]
    fn simulation_is_deterministic() {
        let assumptions = assumptions();
        let calibration = calibration();
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 77,
            continuous_interval_seconds: 1,
        };
        let workload = workloads().remove(1);
        let first = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "irregular",
            30,
            100 * TRILLION,
        )
        .unwrap();
        let second = simulate(
            &policy(),
            &workload,
            &context,
            "test",
            "irregular",
            30,
            100 * TRILLION,
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn zero_activity_still_charges_empty_polls() {
        let assumptions = assumptions();
        let calibration = calibration();
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 1,
        };
        let result = simulate(
            &policy(),
            &workloads()[0],
            &context,
            "test",
            "no_incoming_funding",
            30,
            100 * TRILLION,
        )
        .unwrap();
        assert!(result.total_projected_expenditure_cycles > 0);
        assert_eq!(result.total_funding_cycles, 0);
    }

    #[test]
    fn payout_timing_changes_liquidity_even_when_annual_yield_matches() {
        let assumptions = assumptions();
        let daily =
            funding_schedule("regular_daily", 365 * DAY, &assumptions, &workloads()[1], 1).unwrap();
        let monthly = funding_schedule(
            "regular_monthly",
            365 * DAY,
            &assumptions,
            &workloads()[1],
            1,
        )
        .unwrap();
        assert!(daily.len() > monthly.len());
        assert_ne!(daily.first().unwrap().1, monthly.first().unwrap().1);
    }

    #[test]
    fn delayed_and_interrupted_payouts_are_explicit() {
        let assumptions = assumptions();
        let delayed = funding_schedule(
            "delayed_payout_and_recovery",
            365 * DAY,
            &assumptions,
            &workloads()[1],
            1,
        )
        .unwrap();
        assert!(delayed.first().unwrap().0 >= 90 * DAY);
        let interrupted = funding_schedule(
            "thirty_day_interruption",
            365 * DAY,
            &assumptions,
            &workloads()[1],
            1,
        )
        .unwrap();
        assert!(interrupted.iter().any(|(time, _)| *time == 60 * DAY));
    }

    #[test]
    fn exhaustion_enters_reserve_protection() {
        let mut costly = calibration();
        costly.empty_poll_cycles = TRILLION;
        let assumptions = assumptions();
        let context = SimulationContext {
            calibration: &costly,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 1,
        };
        let result = simulate(
            &policy(),
            &workloads()[0],
            &context,
            "test",
            "no_incoming_funding",
            30,
            2 * TRILLION,
        )
        .unwrap();
        assert!(result.reserve_protection_seconds.is_some());
    }

    #[test]
    fn batch_cost_reproduces_reference_components() {
        let batch = project_batch(
            &test_state(),
            &test_workload(1_000, 0, 1, 0, 0),
            &calibration(),
            1,
            1,
            1,
        )
        .unwrap();
        assert_eq!(batch.processed, 1);
        assert_eq!(batch.global_notifications, 1);
        assert_eq!(batch.specific_notifications, 0);
        assert_eq!(batch.coalesced_notification_dispatches, 1);
        assert_eq!(batch.burn, 1_000_000 + 100_000 + 5_000_000);
    }

    #[test]
    fn irrelevant_blocks_notify_globals_but_not_specific_subscribers() {
        let batch = project_batch(
            &test_state(),
            &test_workload(1_000, 0, 1, 5, 1_000),
            &calibration(),
            1,
            1,
            1,
        )
        .unwrap();
        assert_eq!(batch.global_notifications, 1);
        assert_eq!(batch.specific_notifications, 0);
    }

    #[test]
    fn empty_polls_do_not_notify_globals() {
        let batch = project_batch(
            &test_state(),
            &test_workload(0, 0, 1, 0, 0),
            &calibration(),
            1,
            1,
            1,
        )
        .unwrap();
        assert_eq!(batch.processed, 0);
        assert_eq!(batch.global_notifications, 0);
    }

    #[test]
    fn ten_blocks_in_one_poll_coalesce_one_global_poke() {
        let batch = project_batch(
            &test_state(),
            &test_workload(10_000, 0, 1, 0, 0),
            &calibration(),
            1,
            1,
            1,
        )
        .unwrap();
        assert_eq!(batch.processed, 10);
        assert_eq!(batch.global_notifications, 1);
    }

    #[test]
    fn one_specific_match_does_not_notify_disjoint_subscribers() {
        let batch = project_batch(
            &test_state(),
            &test_workload(1_000, 10_000, 0, 10, 1_000),
            &calibration(),
            1,
            1,
            1,
        )
        .unwrap();
        assert_eq!(batch.processed_specific_matching, 1);
        assert_eq!(batch.specific_notifications, 1);
    }

    #[test]
    fn mixed_notifications_coalesce_by_subscriber_and_poll() {
        let mut workload = test_workload(10_000, 10_000, 2, 3, 10_000);
        workload.global_specific_overlap_subscribers = 2;
        let batch = project_batch(&test_state(), &workload, &calibration(), 1, 1, 1).unwrap();
        assert_eq!(batch.global_notifications, 2);
        assert_eq!(batch.specific_notifications, 3);
        assert_eq!(batch.coalesced_notification_dispatches, 3);
    }

    fn bounded_reference(
        state: &SimState,
        workload: &Workload,
        calibration: &CostCalibration,
        polls: u64,
    ) -> Batch {
        let mut total = Batch::zero();
        let mut backlog = state.backlog;
        let mut matching_backlog = state.specific_matching_backlog;
        let mut block_fraction = state.block_milli_remainder;
        let mut match_fraction = state.specific_matching_basis_remainder;
        let mut subscriber_fraction = state.specific_subscriber_milli_remainder;
        let mut overlap_fraction = state.overlap_notification_remainder;
        for _ in 0..polls {
            block_fraction += u128::from(workload.blocks_per_second_milli);
            let generated = block_fraction / 1_000;
            block_fraction %= 1_000;
            match_fraction +=
                generated * u128::from(workload.specific_matching_blocks_basis_points);
            let generated_matches = match_fraction / 10_000;
            match_fraction %= 10_000;
            backlog += generated;
            matching_backlog += generated_matches;
            let processed = backlog.min(u128::from(workload.assumed_blocks_per_poll_capacity));
            let matches = if backlog == 0 {
                0
            } else {
                matching_backlog * processed / backlog
            };
            backlog -= processed;
            matching_backlog -= matches;
            subscriber_fraction +=
                matches * u128::from(workload.specific_subscribers_per_matching_block_milli);
            let specific =
                (subscriber_fraction / 1_000).min(u128::from(workload.specific_subscribers));
            subscriber_fraction %= 1_000;
            let global = if processed > 0 {
                u128::from(workload.global_subscribers)
            } else {
                0
            };
            let overlap = if workload.specific_subscribers == 0 {
                0
            } else {
                overlap_fraction +=
                    specific * u128::from(workload.global_specific_overlap_subscribers);
                let value = overlap_fraction / u128::from(workload.specific_subscribers);
                overlap_fraction %= u128::from(workload.specific_subscribers);
                value
            };
            let dispatches = global + specific - overlap;
            total.polls += 1;
            total.generated += generated;
            total.processed += processed;
            total.global_notifications += global;
            total.specific_notifications += specific;
            total.coalesced_notification_dispatches += dispatches;
            total.burn += calibration.empty_poll_cycles
                + calibration.cycles_per_processed_block * processed
                + calibration.cycles_per_ledger_page * processed.div_ceil(256).saturating_sub(1)
                + calibration.cycles_per_subscriber_notification * dispatches;
        }
        total
    }

    #[test]
    fn initial_backlog_drains_once_then_remaining_polls_are_empty() {
        let mut state = test_state();
        state.backlog = 1_000;
        let workload = test_workload(0, 0, 1, 0, 0);
        let batch = project_batch(&state, &workload, &calibration(), 1, 100, 1).unwrap();
        assert_eq!(batch.processed, 1_000);
        assert_eq!(batch.global_notifications, 1);
        assert_eq!(batch.coalesced_notification_dispatches, 1);
        assert_eq!(
            batch.burn,
            bounded_reference(&state, &workload, &calibration(), 100).burn
        );
    }

    #[test]
    fn sustained_arrivals_and_burst_coalescing_are_distinct() {
        let steady = project_batch(
            &test_state(),
            &test_workload(1_000, 0, 1, 0, 0),
            &calibration(),
            1,
            100,
            1,
        )
        .unwrap();
        assert_eq!(steady.global_notifications, 100);
        let mut burst_state = test_state();
        burst_state.backlog = 10;
        let burst = project_batch(
            &burst_state,
            &test_workload(0, 0, 1, 0, 0),
            &calibration(),
            1,
            100,
            1,
        )
        .unwrap();
        assert_eq!(burst.global_notifications, 1);
        let idle = project_batch(
            &test_state(),
            &test_workload(0, 0, 1, 0, 0),
            &calibration(),
            1,
            100,
            1,
        )
        .unwrap();
        assert_eq!(idle.global_notifications, 0);
    }

    #[test]
    fn capacity_limited_bursts_match_bounded_per_poll_reference() {
        let mut state = test_state();
        state.backlog = 2_500;
        state.specific_matching_backlog = 1_250;
        let mut workload = test_workload(0, 5_000, 2, 3, 1_000);
        workload.assumed_blocks_per_poll_capacity = 1_000;
        workload.global_specific_overlap_subscribers = 2;
        let projected = project_batch(&state, &workload, &calibration(), 1, 100, 1).unwrap();
        let reference = bounded_reference(&state, &workload, &calibration(), 100);
        assert_eq!(projected.processed, reference.processed);
        assert_eq!(projected.global_notifications, 6);
        assert_eq!(
            projected.specific_notifications,
            reference.specific_notifications
        );
        assert_eq!(
            projected.coalesced_notification_dispatches,
            reference.coalesced_notification_dispatches
        );
        assert_eq!(projected.burn, reference.burn);
    }

    #[test]
    fn burst_then_low_traffic_matches_reference_at_large_balance() {
        let mut state = test_state();
        state.balance = 1_000 * TRILLION;
        state.backlog = 10_000;
        state.specific_matching_backlog = 2_500;
        let mut workload = test_workload(1_000, 2_500, 1, 5, 1_000);
        workload.assumed_blocks_per_poll_capacity = 4_096;
        let projected = project_batch(&state, &workload, &calibration(), 1, 500, 1).unwrap();
        let reference = bounded_reference(&state, &workload, &calibration(), 500);
        assert_eq!(
            projected.global_notifications,
            reference.global_notifications
        );
        assert_eq!(
            projected.specific_notifications,
            reference.specific_notifications
        );
        assert_eq!(projected.burn, reference.burn);
    }

    #[test]
    fn independent_price_rounding_and_multiplier_validation() {
        let basis = 6_250;
        assert_eq!(
            rounded_admission_price(10 * ICP_E8S, basis).unwrap(),
            7 * ICP_E8S
        );
        assert_eq!(
            rounded_admission_price(20 * ICP_E8S, basis).unwrap(),
            13 * ICP_E8S
        );
        assert_eq!(
            rounded_admission_price(1_000 * ICP_E8S, basis).unwrap(),
            625 * ICP_E8S
        );
        assert!(rounded_admission_price(10 * ICP_E8S, 0).is_err());
        assert!(rounded_admission_price(10 * ICP_E8S, 10_001).is_err());
        let mut config = default_model_config();
        config.workloads[0]
            .productive_endowment_backing
            .admission_price_basis_points_of_reference = 12_500;
        assert!(validate_model_config(&config).is_err());
    }

    #[test]
    fn generic_instance_does_not_inherit_canonical_protocol_endowment() {
        let assumptions = assumptions();
        let mut workloads = workloads();
        let generic_before = annual_payout_e8s(&assumptions, &workloads[2], 560).unwrap();
        workloads[1]
            .productive_endowment_backing
            .protocol_endowment_icp_e8s = 10_000 * ICP_E8S;
        assert_eq!(
            generic_before,
            annual_payout_e8s(&assumptions, &workloads[2], 560).unwrap()
        );
        assert_eq!(
            workloads[2]
                .productive_endowment_backing
                .protocol_endowment_icp_e8s,
            0
        );
        assert!(generic_before > 0);
    }

    #[test]
    fn recurring_maintenance_is_charged_once_per_lane_run() {
        let assumptions = assumptions();
        let calibration = calibration();
        let context = SimulationContext {
            calibration: &calibration,
            assumptions: &assumptions,
            seed: 1,
            continuous_interval_seconds: 10,
        };
        let result = simulate(
            &policy(),
            &workloads()[0],
            &context,
            "test",
            "no_incoming_funding",
            1,
            25 * TRILLION,
        )
        .unwrap();
        assert_eq!(result.funding_maintenance_runs, 24);
        assert_eq!(result.pricing_maintenance_runs, 1);
        assert_eq!(result.total_maintenance_cycles, 24 * 20_000 + 30_000);
        assert_eq!(
            result.total_projected_expenditure_cycles,
            result.total_ordinary_polling_cycles + result.total_maintenance_cycles
        );
    }

    #[test]
    fn tracked_model_config_matches_defaults() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/model-config.json");
        assert_eq!(read_model_config(&path).unwrap(), default_model_config());
    }
}
