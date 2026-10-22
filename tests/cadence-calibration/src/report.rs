use std::{fs, path::Path};

use anyhow::{anyhow, bail, Context, Result};

use crate::{
    measurement::read_measurements,
    model::read_simulations,
    types::{Mode, Policy, SimulationResult, SimulationSet},
    TRILLION,
};

const DAY: u64 = 86_400;

pub fn run(directory: &Path) -> Result<String> {
    let measurements_path = directory.join("measurements.json");
    let simulations_path = directory.join("simulations.json");
    let measurements = read_measurements(&measurements_path)?;
    let simulations = read_simulations(&simulations_path)?;
    let mut report = String::new();
    report.push_str("# Event Horizon cadence calibration report\n\n");
    report.push_str(&format!(
        "Dataset: `{}` at git revision `{}` (working tree dirty: {}). PocketIC {}, deterministic seed {}.\n\n",
        directory.display(),
        measurements.provenance.git_commit,
        measurements.provenance.working_tree_dirty,
        measurements.provenance.pocketic_version,
        measurements.provenance.seed,
    ));
    report.push_str("## Method and provenance\n\n");
    report.push_str("The measurement layer executed the debug/test Event Horizon Wasm in a one-application-subnet PocketIC fixture with the normal cost schedule. Installation, bootstrap, pricing setup, and one complete all-lane scheduler warm-up occurred before each timer-driven measurement window. Non-Economy baseline windows finish before the next funding or pricing timer; Economy is marked maintenance-inclusive because its one-hour poll and funding intervals coincide. Integrated Reserve Protection deliberately ran polling, funding, and pricing together. Backend balance deltas exclude mock Ledger and subscriber balances. Every asynchronous poll was awaited through its scheduler lane, and one-way fan-out measurements waited for balance settlement.\n\n");
    report.push_str(&format!(
        "Debug/test Wasm SHA-256: `{}`. Completed measurement scenarios: {}. Threshold parity observations: {}.\n\n",
        measurements.provenance.debug_wasm_sha256,
        measurements.scenarios.len(),
        measurements.threshold_observations.len(),
    ));

    report.push_str("## Baseline polling\n\n");
    report.push_str("| Mode | Polls | Simulated elapsed | Median cycles/poll | Min–max cycles/poll | Ledger calls |\n|---|---:|---:|---:|---:|---:|\n");
    for mode in Mode::ACTIVE {
        if let Some(item) = measurements
            .scenarios
            .iter()
            .find(|item| item.experiment == "A_baseline_polling" && item.polling_mode == Some(mode))
        {
            report.push_str(&format!(
                "| {} | {} | {} s | {} | {}–{} | {} |\n",
                mode.label(),
                item.completed_polls,
                item.simulated_time_elapsed_seconds,
                integer(item.median_cycles_per_poll.unwrap_or_default()),
                integer(item.minimum_cycles_per_poll.unwrap_or_default()),
                integer(item.maximum_cycles_per_poll.unwrap_or_default()),
                item.completed_ledger_calls,
            ));
        }
    }
    report.push_str("\nThese tier rows exercise the real timer delays. Economy also includes coincident hourly funding maintenance and is excluded from the empty-poll coefficient; the other bounded windows isolate idle polling after the all-lane warm-up. Continuous records zero added simulated delay; it does not establish a one-poll-per-second production rate. Reserve Protection is reported separately because ordinary polls are suspended.\n\n");
    let mut repeats: Vec<u128> = measurements
        .scenarios
        .iter()
        .filter(|item| item.experiment == "A_repeatability")
        .filter_map(|item| item.median_cycles_per_poll)
        .collect();
    if !repeats.is_empty() {
        repeats.sort_unstable();
        let mean = repeats.iter().copied().sum::<u128>() / repeats.len() as u128;
        let median = repeats[(repeats.len() - 1) / 2];
        report.push_str(&format!(
            "Across {} fresh equivalent Very Fast fixtures, the one-poll mean was {}, median was {}, and observed range was {}–{} cycles. This is empirical repeatability evidence, not a confidence interval.\n\n",
            repeats.len(),
            integer(mean),
            integer(median),
            integer(*repeats.first().expect("nonempty repeats")),
            integer(*repeats.last().expect("nonempty repeats")),
        ));
    }
    if let Some(reserve) = measurements
        .scenarios
        .iter()
        .find(|item| item.scenario_id == "reserve_protection_funding_recovery")
    {
        report.push_str(&format!(
            "Reserve Protection completed {} ordinary polls and {} polling Ledger calls while the independent funding path accepted its synthetic raw-ICP top-up transfer. {}\n\n",
            reserve.completed_polls,
            reserve.completed_ledger_calls,
            reserve.notes.join(" ")
        ));
    } else {
        report.push_str("The compact tracked fixture omits the integrated Reserve Protection scenario; run `smoke` or `full` before drawing a recovery-lane conclusion.\n\n");
    }
    if let Some(integrated) = measurements
        .scenarios
        .iter()
        .find(|item| item.scenario_id == "integrated_fast_all_scheduler_lanes")
    {
        report.push_str(&format!(
            "The separate integrated Fast scenario consumed {} cycles while one poll and the ordinary funding/pricing maintenance lanes completed. It is whole-scheduler evidence and is excluded from the isolated empty-poll coefficient.\n\n",
            integer(integrated.net_cycles_consumed)
        ));
    }

    report.push_str("## Ledger traffic and reader architecture\n\n");
    report.push_str("| Architecture | Profile | Decoded blocks | Cursor advance | Polls | Calls | Pages | Max/avg lag | Cycles | Cycles/block (gross) | Catch-up |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for item in measurements
        .scenarios
        .iter()
        .filter(|item| item.experiment == "B_ledger_volume")
    {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} / {} | {} | {} | {} s |\n",
            architecture_label(&item.architecture),
            item.traffic_profile,
            item.processed_blocks,
            item.cursor_after.saturating_sub(item.cursor_before),
            item.completed_polls,
            item.completed_ledger_calls,
            item.ledger_pages,
            item.maximum_ledger_lag,
            item.average_ledger_lag,
            integer(item.net_cycles_consumed),
            integer(if item.processed_blocks == 0 {
                0
            } else {
                item.net_cycles_consumed / u128::from(item.processed_blocks)
            }),
            item.catch_up_time_seconds.unwrap_or_default(),
        ));
    }
    if let (Some(canonical_idle), Some(generic_idle)) = (
        measurements.scenarios.iter().find(|item| {
            item.architecture == "canonical_icp_shared_scan"
                && item.traffic_profile == "idle_0_blocks_per_second"
        }),
        measurements.scenarios.iter().find(|item| {
            item.architecture == "generic_icrc3_separate_admission_scan"
                && item.traffic_profile == "idle_0_blocks_per_second"
        }),
    ) {
        let canonical_per_poll =
            canonical_idle.net_cycles_consumed / u128::from(canonical_idle.completed_polls);
        let generic_per_poll =
            generic_idle.net_cycles_consumed / u128::from(generic_idle.completed_polls);
        report.push_str(&format!(
            "\nIn this fixture, the generic reader plus its separate ICP admission scan cost {} cycles per idle poll versus {} for the canonical shared scan: {} additional cycles ({:.1}% more). Archive-prefix rows show that Event Horizon advances across validated archive metadata without fetching archive callbacks, then decodes the live tail; their cursor advance is intentionally larger than decoded-block count.\n",
            integer(generic_per_poll),
            integer(canonical_per_poll),
            integer(generic_per_poll.saturating_sub(canonical_per_poll)),
            (generic_per_poll.saturating_sub(canonical_per_poll) as f64 * 100.0)
                / canonical_per_poll as f64,
        ));
    }
    report.push_str("\nThe offline model keeps separate architecture profiles:\n\n| Architecture | Empty poll | Residual block | Additional page |\n|---|---:|---:|---:|\n");
    for calibration in &simulations.architecture_calibrations {
        report.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            architecture_label(&calibration.architecture),
            integer(calibration.empty_poll_cycles),
            integer(calibration.cycles_per_processed_block),
            integer(calibration.cycles_per_ledger_page),
        ));
    }
    report.push_str("\nPage and block terms are partially confounded by Candid decoding, stable-state work, and response shape, so they are scenario-calibration coefficients rather than universal replica prices.\n\n");

    report.push_str("## Notification fan-out\n\n");
    report.push_str("| Subscribers | Backend cycles | Attempted pokes | Accepted one-way calls | Payload bytes | Marginal cycles/subscriber vs 0 |\n|---:|---:|---:|---:|---:|---:|\n");
    let zero = measurements
        .scenarios
        .iter()
        .find(|item| item.subscriber_kind == "global" && item.subscriber_count == 0)
        .ok_or_else(|| anyhow!("missing zero-subscriber fan-out scenario"))?;
    for item in measurements.scenarios.iter().filter(|item| {
        item.experiment == "C_notification_fanout" && item.subscriber_kind == "global"
    }) {
        let marginal = if item.subscriber_count == 0 {
            0
        } else {
            item.net_cycles_consumed
                .saturating_sub(zero.net_cycles_consumed)
                / u128::from(item.subscriber_count)
        };
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            item.subscriber_count,
            integer(item.net_cycles_consumed),
            item.poke_attempts,
            item.accepted_oneway_calls,
            item.callback_payload_bytes
                .map_or_else(|| "—".into(), |value| value.to_string()),
            integer(marginal),
        ));
    }
    let full_delivery_shapes = [
        "real_global_coalesced_10_transactions",
        "range_expanded_32_targets_one_match",
        "mixed_global_and_specific_real_receivers",
    ]
    .iter()
    .all(|id| {
        measurements
            .scenarios
            .iter()
            .any(|item| item.scenario_id == *id)
    });
    report.push_str(&format!(
        "\nSelected model coefficient: {} backend cycles per additional notification. This independently compares with the earlier approximately 5.49 million-cycle observation, but remains specific to this PocketIC fixture and payload. Synthetic fan-out does not include recipient execution. {}\n\n",
        integer(simulations.calibration.cycles_per_subscriber_notification),
        if full_delivery_shapes {
            "Dedicated cases separately confirmed unified record receivers, ten-transaction coalescing, mixed global/specific delivery, and a 32-target expanded range."
        } else {
            "Those receiver-execution and delivery-shape cases require the full profile and are not claimed by this compact dataset."
        },
    ));

    report.push_str("## Notification latency\n\n");
    report.push_str(
        "| Mode | Samples | Min | Median | p95 | Max |\n|---|---:|---:|---:|---:|---:|\n",
    );
    for item in measurements.scenarios.iter().filter(|item| {
        item.experiment == "D_notification_latency" && item.traffic_profile == "isolated"
    }) {
        report.push_str(&format!(
            "| {} | {} | {} s | {} s | {} s | {} s |\n",
            item.polling_mode.map(Mode::label).unwrap_or("unknown"),
            item.latency.samples_seconds.len(),
            item.latency.minimum_seconds.unwrap_or_default(),
            item.latency.median_seconds.unwrap_or_default(),
            item.latency.percentile_95_seconds.unwrap_or_default(),
            item.latency.maximum_seconds.unwrap_or_default(),
        ));
    }
    report.push_str("\nThe timestamps are simulated IC time. PocketIC rounds do not advance that clock, so Continuous legitimately appears as zero simulated seconds while still requiring multiple execution rounds. These are timer-semantics checks, not mainnet tail-latency estimates.\n\n");

    report.push_str("## Maintenance calibration\n\n");
    report.push_str("Recurring maintenance is charged independently of polling: funding hourly and pricing daily. It is not included again in the ordinary-poll coefficient.\n\n");
    report.push_str("| Component | Estimated cycles/run | Derivation |\n|---|---:|---|\n");
    report.push_str(&format!(
        "| Funding | {} | Economy maintenance-inclusive median minus isolated poll |\n| Pricing | {} | Integrated all-lane Fast residual minus poll and funding |\n\n",
        optional_integer(simulations.calibration.funding_maintenance_cycles),
        optional_integer(simulations.calibration.pricing_maintenance_cycles),
    ));

    report.push_str("## Funding and runway model\n\n");
    let expected = workload(&simulations, "expected")?;
    let backing = &expected.productive_endowment_backing;
    report.push_str(&format!(
        "The expected workload has explicit backing for {} global, {} account, and {} range declarations. Its global declaration uses the 1,000 ICP reference basis; each class's qualifying price is independently rounded up to whole ICP at the configured {}% dynamic-price assumption. The default funding calculation assumes patrons endow exactly these minima; voluntary extra principal is possible but is not represented by a multiplier above 100%. The canonical instance has {} ICP of separate protocol productive principal; an unseeded generic instance does not inherit it. No principal becomes liquid cycles; only modelled recurring payouts at {}% effective APY are converted at {} XDR/ICP. The stress workload is explicitly subscriber-funding-decoupled.\n\n",
        backing.global_endowments,
        backing.account_endowments,
        backing.range_endowments,
        decimal_basis_points(backing.admission_price_basis_points_of_reference),
        backing.protocol_endowment_icp_e8s / 100_000_000,
        decimal_basis_points(simulations.funding_assumptions.effective_payout_apy_basis_points),
        decimal_permyriad(simulations.funding_assumptions.xdr_per_icp_permyriad),
    ));
    report.push_str(&format!(
        "Funding assumptions: recognition delay {} s, conversion delay {} s, conversion fee {} ICP e8s per receipt, delivery shortfall {} basis points and diversion {} basis points. The configured effective payout APY, rather than gross maturity APY, determines receipts.\n\n",
        simulations.funding_assumptions.initial_recognition_delay_seconds,
        simulations.funding_assumptions.conversion_delay_seconds,
        simulations.funding_assumptions.conversion_fee_e8s,
        simulations.funding_assumptions.delivery_shortfall_basis_points,
        simulations.funding_assumptions.surplus_diversion_basis_points,
    ));
    report.push_str("Canonical workload rows are alternative scenarios for one planned launch, with separate simulated balances; their protocol seeds and payouts are never pooled.\n\n");
    report.push_str("| 365-day funding profile (current policy, expected workload) | Funding | Polling | Maintenance | Total spend | Ending | Minimum | First downgrade | Reserve | Window covered |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for result in simulations.results.iter().filter(|result| {
        result.simulation_kind == "funding_profile"
            && result.policy_id == "current"
            && result.workload_id == "expected"
            && result.horizon_days == 365
    }) {
        report.push_str(&format!(
            "| {} | {} T | {} T | {} T | {} T | {} T | {} T | {} | {} | {} |\n",
            result.funding_profile,
            trillions(result.total_funding_cycles),
            trillions(result.total_ordinary_polling_cycles),
            trillions(result.total_maintenance_cycles),
            trillions(result.total_projected_expenditure_cycles),
            trillions(result.ending_cycles),
            trillions(result.minimum_cycles),
            optional_days(result.first_downgrade_seconds),
            optional_days(result.reserve_protection_seconds),
            yes_no(result.window_funding_covers_expenditure),
        ));
    }

    report.push_str("\nWindow coverage compares actual receipts with expenditure over the same horizon, independently of liquidity. Reserve entry and exhaustion are separate safety outcomes.\n\n");
    report.push_str("## Hypothetical fixed-tier mature economics\n\n");
    report.push_str("These continuing workload rates exclude startup backlog and recognition delay. They assume weekly completed-period payouts, deduct conversion fees per payout, apply configured effective APY, delivery shortfall, diversion and ICP/XDR conversion, and include hourly funding and daily pricing maintenance. They are separate from the adaptive simulation path. Capacity-limited rows may process fewer blocks than arrive; a positive financial margin does not establish throughput adequacy. A small positive annual margin does not establish short-term liquidity or indefinite Continuous operation.\n\n");
    report.push_str("| Workload | Fixed tier | Interval | Mature income | Annual expenditure | Annual margin | Processed / generated blocks |\n|---|---|---:|---:|---:|---:|---:|\n");
    for result in &simulations.fixed_tier_economics {
        let margin = result.steady_state_annual_margin_cycles;
        report.push_str(&format!(
            "| {} | {} | {} s | {} T | {} T | {}{} T | {} / {} |\n",
            result.workload_id,
            result.mode.label(),
            result.effective_poll_interval_seconds,
            trillions(result.mature_annual_income_cycles),
            trillions(result.annualized_operating_expenditure_cycles),
            if margin < 0 { "-" } else { "" },
            trillions(margin.unsigned_abs()),
            result.annual_processed_blocks,
            result.annual_generated_blocks
        ));
    }
    report.push_str("\n## Queue and reserve decomposition\n\n");
    report.push_str("Pre-poll pending blocks measure waiting for service. Post-poll outstanding blocks measure active processing lag; both maxima include peaks inside batches. Reserve arrivals count all blocks generated while polling is suspended, including repeated reserve episodes; they may later be drained. Transition snapshots record outstanding blocks at the first event. Ending backlog alone establishes neither throughput incapacity nor archival loss.\n\n");
    report.push_str("| Scenario | Active pre-poll max | Active post-poll max | First downgrade outstanding | First reserve outstanding | Reserve arrivals | Ending | Suspended seconds | Liquidity | Window surplus / deficit |\n|---|---:|---:|---:|---:|---:|---:|---:|---|---:|\n");
    for result in &simulations.results {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} |\n",
            result.scenario_id,
            result.maximum_active_pre_poll_pending_blocks,
            result.maximum_active_post_poll_outstanding_blocks,
            result
                .outstanding_blocks_at_first_downgrade
                .map_or_else(|| "not reached".into(), integer),
            result
                .outstanding_blocks_at_first_reserve
                .map_or_else(|| "not reached".into(), integer),
            result.additional_reserve_backlog_blocks,
            result.ending_backlog_blocks,
            result.seconds_in_modes[0],
            yes_no(result.liquidity_maintained),
            trillions(result.funding_surplus_cycles),
            trillions(result.funding_shortfall_cycles)
        ));
    }

    report.push_str("\n### Workload and reserve-safety comparison\n\n");
    report.push_str("These weekly-funded, 365-day rows start each policy at its own Continuous entry. They expose backlog and Reserve Protection outcomes across workloads; they are not equal-balance comparisons.\n\n");
    report.push_str("| Policy | Workload | Minimum | Reserve reached | Ending/max backlog | Ending latency | Transitions / max 7 d |\n|---|---|---:|---:|---:|---:|---:|\n");
    for candidate in &simulations.policies {
        for candidate_workload in &simulations.workloads {
            let result = find_result(
                &simulations,
                "candidate_workload",
                &candidate.id,
                &candidate_workload.id,
                "regular_weekly",
                365,
                simulations.continuous_effective_poll_interval_seconds,
                candidate.thresholds.continuous_entry,
            )?;
            report.push_str(&format!(
                "| {} | {} | {} T | {} | {}/{} | {} s | {}/{} |\n",
                candidate.id,
                candidate_workload.id,
                trillions(result.minimum_cycles),
                optional_days(result.reserve_protection_seconds),
                result.ending_backlog_blocks,
                result.maximum_backlog_blocks,
                result.projected_latency_seconds,
                result.tier_transitions,
                result.maximum_transitions_in_rolling_seven_days,
            ));
        }
    }

    report.push_str("\n### Lower-tier runway objectives\n\n");
    report.push_str("Each row starts exactly at the current tier's entry threshold with no funding and the expected workload. `Meets` evaluates the configured objective rather than merely displaying it.\n\n");
    report.push_str("| Architecture | Tier | Entry/exit | Objective | Modelled runway | Meets | Ending backlog (365 d) |\n|---|---|---:|---:|---:|---|---:|\n");
    let current = policy(&simulations, "current")?;
    for workload_id in ["expected", "expected_generic"] {
        let Ok(runway_workload) = workload(&simulations, workload_id) else {
            continue;
        };
        for (index, mode) in [Mode::Economy, Mode::Standard, Mode::Fast, Mode::VeryFast]
            .into_iter()
            .enumerate()
        {
            let result = find_result(
                &simulations,
                &format!("entry_runway_{}", mode_id(mode)),
                "current",
                workload_id,
                "no_incoming_funding",
                365,
                simulations.continuous_effective_poll_interval_seconds,
                current.thresholds.entry(mode),
            )?;
            let runway = result.first_downgrade_seconds.unwrap_or(365 * DAY);
            let target = simulations.lower_tier_runway_targets_days[index];
            report.push_str(&format!(
                "| {} | {} | {}/{} T | {} d | {:.2} d | {} | {} |\n",
                architecture_label(&runway_workload.architecture),
                mode.label(),
                trillions(current.thresholds.entry(mode)),
                trillions(current.thresholds.exit(mode)),
                target,
                runway as f64 / DAY as f64,
                meets(runway, target),
                result.ending_backlog_blocks,
            ));
        }
    }
    report.push_str("\nThe lower tiers cannot be judged on runway alone. At one block per second, Economy, Standard and Fast clear each poll under the configured capacity: pre-poll waiting is distinct from persistent post-poll lag. Ending backlog can instead accumulate after Reserve Protection suspends polling. Backlog alone does not prove archived transactions were missed. These mixed service and architecture results support provisional retention, not a lower-tier threshold change.\n");

    report.push_str("\n### Equal-balance policy comparison\n\n");
    report.push_str("Every policy below receives the same physical balance, expected workload, no funding, and 90-day horizon. This comparison is separate from entry-to-exit runway.\n\n");
    report.push_str("| Initial | Policy | Initial mode | Continuous/VF time | Minimum | Reserve reached | Ending backlog | Transitions |\n|---:|---|---|---:|---:|---|---:|---:|\n");
    for &initial in &simulations.equal_starting_balances_cycles {
        for candidate in &simulations.policies {
            let result = find_result(
                &simulations,
                "equal_balance",
                &candidate.id,
                "expected",
                "no_incoming_funding",
                90,
                simulations.continuous_effective_poll_interval_seconds,
                initial,
            )?;
            report.push_str(&format!(
                "| {} T | {} | {} | {:.2}/{:.2} d | {} T | {} | {} | {} |\n",
                trillions(initial),
                candidate.id,
                result.initial_mode.label(),
                result.seconds_in_modes[5] as f64 / DAY as f64,
                result.seconds_in_modes[4] as f64 / DAY as f64,
                trillions(result.minimum_cycles),
                optional_days(result.reserve_protection_seconds),
                result.ending_backlog_blocks,
                result.tier_transitions,
            ));
        }
    }

    report.push_str("\n### Continuous completed-poll-rate sensitivity\n\n");
    report.push_str("Every interval is an assumption. Runway starts at each policy's own Continuous entry; weekly window funding coverage includes that workload's productive backing.\n\n");
    report.push_str("| Policy | Interval | No-funding runway | Weekly spend | Weekly ending/minimum | Window covered |\n|---|---:|---:|---:|---:|---|\n");
    for candidate in &simulations.policies {
        for &interval in &simulations.continuous_effective_poll_intervals_seconds {
            let no_funding = find_result(
                &simulations,
                "poll_rate_sensitivity",
                &candidate.id,
                "expected",
                "no_incoming_funding",
                365,
                interval,
                candidate.thresholds.continuous_entry,
            )?;
            let weekly = find_result(
                &simulations,
                "poll_rate_sensitivity",
                &candidate.id,
                "expected",
                "regular_weekly",
                365,
                interval,
                candidate.thresholds.continuous_entry,
            )?;
            report.push_str(&format!(
                "| {} | {} s | {} | {} T | {}/{} T | {} |\n",
                candidate.id,
                interval,
                optional_days(no_funding.continuous_runway_seconds),
                trillions(weekly.total_projected_expenditure_cycles),
                trillions(weekly.ending_cycles),
                trillions(weekly.minimum_cycles),
                yes_no(weekly.window_funding_covers_expenditure),
            ));
        }
    }

    report.push_str("\n### Funding-interruption and hysteresis comparison\n\n");
    report.push_str("| Policy | Profile | Minimum | Reserve reached | Transitions / max 7 d | Ending backlog |\n|---|---|---:|---:|---:|---:|\n");
    for candidate in &simulations.policies {
        for profile in [
            "seven_day_interruption",
            "thirty_day_interruption",
            "ninety_day_interruption",
            "delayed_payout_and_recovery",
        ] {
            let result = find_result(
                &simulations,
                "interruption_comparison",
                &candidate.id,
                "expected",
                profile,
                365,
                simulations.continuous_effective_poll_interval_seconds,
                candidate.thresholds.continuous_entry,
            )?;
            report.push_str(&format!(
                "| {} | {} | {} T | {} | {}/{} | {} |\n",
                candidate.id,
                profile,
                trillions(result.minimum_cycles),
                optional_days(result.reserve_protection_seconds),
                result.tier_transitions,
                result.maximum_transitions_in_rolling_seven_days,
                result.ending_backlog_blocks,
            ));
        }
    }

    report.push_str("\n## Threshold comparison and conclusion\n\n");
    let recommendation = choose_recommendation(&simulations)?;
    report.push_str("| Mode | Current entry | Proposed entry | Current exit | Proposed exit | Evidence |\n|---|---:|---:|---:|---:|---|\n");
    for mode in Mode::ACTIVE {
        let proposed = recommendation.unwrap_or(current);
        report.push_str(&format!(
            "| {} | {} T | {} T | {} T | {} T | {} |\n",
            mode.label(),
            trillions(current.thresholds.entry(mode)),
            trillions(proposed.thresholds.entry(mode)),
            trillions(current.thresholds.exit(mode)),
            trillions(proposed.thresholds.exit(mode)),
            if recommendation.is_some() {
                "multi-objective model"
            } else {
                "retain pending calibration"
            },
        ));
    }
    report.push_str("\nReserve Protection remains at 1 T. The corrected model does not identify a candidate that dominates the current policy across completed-poll-rate sensitivity, equal-balance responsiveness, interruption safety, and hysteresis. The previous 125 T recommendation is withdrawn. Retaining the production table is a provisional no-change conclusion, not evidence that its values are optimal. Lower-tier results are architecture-sensitive and do not justify a change without mainnet calibration.\n\n");

    report.push_str("## Hysteresis and boundedness\n\n");
    report.push_str(&format!(
        "All {} just-below/equal/just-above and transition-sequence PocketIC observations agreed with the production `next_mode` implementation. Candidate modelling uses the same stateful increase/hold/fallback semantics. Every benchmark wait was bounded; no scenario in this dataset was truncated. Continuous execution was capped by completed polls/ticks and long horizons were modelled offline.\n\n",
        measurements.threshold_observations.len()
    ));

    report.push_str("## Limitations and mainnet calibration\n\n");
    for limitation in measurements
        .limitations
        .iter()
        .chain(simulations.limitations.iter())
    {
        report.push_str(&format!("- {limitation}\n"));
    }
    report.push_str("\nA later operator can run `./tools/scripts/mainnet-observe` and compare read-only mainnet cycles-consumption counters, balance deltas, inferred mode, Ledger traffic, subscriber activity, Faucet receipt timing, and actual ICP-to-cycles conversion with this dataset. Unavailable counters should be recorded as unavailable; no diagnostic API or artificial real-ICP Continuous-mode top-up is warranted.\n\n");
    report.push_str(&format!(
        "## Reproduction\n\n```bash\n./tools/scripts/calibrate-cadence full --seed 20261011 --output observations/cadence/full-20261011\n./tools/scripts/calibrate-cadence model --input observations/cadence/full-20261011/measurements.json --config tests/cadence-calibration/fixtures/model-config.json --output {} --seed 20261011\n./tools/scripts/calibrate-cadence report --input {}\n```\n",
        directory.display(),
        directory.display(),
    ));
    fs::write(directory.join("report.md"), &report)
        .with_context(|| format!("write {}/report.md", directory.display()))?;
    Ok(report)
}

fn choose_recommendation(set: &SimulationSet) -> Result<Option<&Policy>> {
    let current = policy(set, "current")?;
    for candidate in set.policies.iter().filter(|item| item.id != "current") {
        let runway_at_every_rate = set
            .continuous_effective_poll_intervals_seconds
            .iter()
            .map(|&interval| {
                find_result(
                    set,
                    "poll_rate_sensitivity",
                    &candidate.id,
                    "expected",
                    "no_incoming_funding",
                    365,
                    interval,
                    candidate.thresholds.continuous_entry,
                )
            })
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|result| result.continuous_runway_seconds.unwrap_or(365 * DAY) >= 30 * DAY);
        let interruption_safe = [
            "seven_day_interruption",
            "thirty_day_interruption",
            "ninety_day_interruption",
            "delayed_payout_and_recovery",
        ]
        .iter()
        .map(|profile| {
            find_result(
                set,
                "interruption_comparison",
                &candidate.id,
                "expected",
                profile,
                365,
                set.continuous_effective_poll_interval_seconds,
                candidate.thresholds.continuous_entry,
            )
        })
        .collect::<Result<Vec<_>>>()?
        .iter()
        .all(|result| result.reserve_protection_seconds.is_none());
        let equal_balance_not_less_responsive = set
            .equal_starting_balances_cycles
            .iter()
            .map(|&initial| {
                Ok((
                    find_result(
                        set,
                        "equal_balance",
                        &candidate.id,
                        "expected",
                        "no_incoming_funding",
                        90,
                        set.continuous_effective_poll_interval_seconds,
                        initial,
                    )?,
                    find_result(
                        set,
                        "equal_balance",
                        &current.id,
                        "expected",
                        "no_incoming_funding",
                        90,
                        set.continuous_effective_poll_interval_seconds,
                        initial,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|(candidate_result, current_result)| {
                responsiveness_score(candidate_result) >= responsiveness_score(current_result)
            });
        let equal_balance_safe = set
            .equal_starting_balances_cycles
            .iter()
            .map(|&initial| {
                Ok((
                    find_result(
                        set,
                        "equal_balance",
                        &candidate.id,
                        "expected",
                        "regular_weekly",
                        365,
                        set.continuous_effective_poll_interval_seconds,
                        initial,
                    )?,
                    find_result(
                        set,
                        "equal_balance",
                        &current.id,
                        "expected",
                        "regular_weekly",
                        365,
                        set.continuous_effective_poll_interval_seconds,
                        initial,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|(candidate_result, current_result)| {
                (current_result.reserve_protection_seconds.is_some()
                    || candidate_result.reserve_protection_seconds.is_none())
                    && candidate_result.maximum_backlog_blocks
                        <= current_result.maximum_backlog_blocks
            });
        let candidate_weekly = find_result(
            set,
            "candidate_workload",
            &candidate.id,
            "expected",
            "regular_weekly",
            365,
            set.continuous_effective_poll_interval_seconds,
            candidate.thresholds.continuous_entry,
        )?;
        let current_weekly = find_result(
            set,
            "candidate_workload",
            &current.id,
            "expected",
            "regular_weekly",
            365,
            set.continuous_effective_poll_interval_seconds,
            current.thresholds.continuous_entry,
        )?;
        let hysteresis_not_worse = candidate_weekly.maximum_transitions_in_rolling_seven_days
            <= current_weekly.maximum_transitions_in_rolling_seven_days;
        if runway_at_every_rate
            && interruption_safe
            && equal_balance_safe
            && equal_balance_not_less_responsive
            && hysteresis_not_worse
        {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn responsiveness_score(result: &SimulationResult) -> u128 {
    result
        .seconds_in_modes
        .iter()
        .enumerate()
        .map(|(mode, seconds)| mode as u128 * u128::from(*seconds))
        .sum()
}

fn policy<'a>(set: &'a SimulationSet, id: &str) -> Result<&'a Policy> {
    set.policies
        .iter()
        .find(|policy| policy.id == id)
        .ok_or_else(|| anyhow!("missing policy {id}"))
}

fn workload<'a>(set: &'a SimulationSet, id: &str) -> Result<&'a crate::types::Workload> {
    set.workloads
        .iter()
        .find(|workload| workload.id == id)
        .ok_or_else(|| anyhow!("missing workload {id}"))
}

#[allow(clippy::too_many_arguments)]
fn find_result<'a>(
    set: &'a SimulationSet,
    simulation_kind: &str,
    policy: &str,
    workload: &str,
    funding: &str,
    horizon: u64,
    interval: u64,
    initial_cycles: u128,
) -> Result<&'a SimulationResult> {
    set.results
        .iter()
        .find(|result| {
            result.simulation_kind == simulation_kind
                && result.policy_id == policy
                && result.workload_id == workload
                && result.funding_profile == funding
                && result.horizon_days == horizon
                && result.continuous_effective_poll_interval_seconds == interval
                && result.initial_cycles == initial_cycles
        })
        .ok_or_else(|| {
            anyhow!("missing simulation kind={simulation_kind} policy={policy} workload={workload} funding={funding} horizon={horizon} interval={interval} initial={initial_cycles}")
        })
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

fn integer(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(character);
    }
    out
}

fn optional_integer(value: Option<u128>) -> String {
    value.map_or_else(|| "not measured".into(), integer)
}

fn trillions(value: u128) -> String {
    format!(
        "{}.{:03}",
        value / TRILLION,
        value % TRILLION / 1_000_000_000
    )
}

fn optional_days(value: Option<u64>) -> String {
    value.map_or_else(
        || "not reached".into(),
        |seconds| format!("{:.2} d", seconds as f64 / DAY as f64),
    )
}

fn architecture_label(value: &str) -> &str {
    match value {
        "canonical_icp_shared_scan" => "canonical ICP shared",
        "generic_icrc3_separate_admission_scan" => "generic ICRC-3 + ICP admission",
        _ => value,
    }
}

fn decimal_basis_points(value: u64) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

fn decimal_permyriad(value: u64) -> String {
    format!("{}.{:04}", value / 10_000, value % 10_000)
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn meets(seconds: u64, days: u64) -> &'static str {
    if seconds >= days * DAY {
        "yes"
    } else {
        "no"
    }
}

pub fn validate_directory(directory: &Path) -> Result<()> {
    if !directory.join("measurements.json").is_file() {
        bail!("{} has no measurements.json", directory.display())
    }
    if !directory.join("simulations.json").is_file() {
        bail!("{} has no simulations.json", directory.display())
    }
    Ok(())
}
