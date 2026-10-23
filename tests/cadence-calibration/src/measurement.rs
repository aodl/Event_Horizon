use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use candid::{decode_one, encode_one, CandidType, Deserialize, Nat, Principal};
use pocket_ic::{CreateCanisterParams, PocketIc, PocketIcBuilder};
use sha2::{Digest, Sha256};

use crate::{
    types::{
        ExecutionLimits, LatencySummary, Measurement, MeasurementSet, Mode, Provenance,
        ThresholdObservation,
    },
    TRILLION,
};

static EVENT_HORIZON_WASM: OnceLock<Vec<u8>> = OnceLock::new();
static LEDGER_WASM: OnceLock<Vec<u8>> = OnceLock::new();
static ICRC3_LEDGER_WASM: OnceLock<Vec<u8>> = OnceLock::new();
static HISTORIAN_WASM: OnceLock<Vec<u8>> = OnceLock::new();
static CMC_WASM: OnceLock<Vec<u8>> = OnceLock::new();
static SUBSCRIBER_WASM: OnceLock<Vec<u8>> = OnceLock::new();

const ICP_E8S: u64 = 100_000_000;

#[derive(Clone, Debug)]
pub struct RunConfig {
    pub seed: u64,
    pub output: PathBuf,
    pub limits: ExecutionLimits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Architecture {
    CanonicalIcp,
    GenericIcrc3,
}

impl Architecture {
    const fn id(self) -> &'static str {
        match self {
            Self::CanonicalIcp => "canonical_icp_shared_scan",
            Self::GenericIcrc3 => "generic_icrc3_separate_admission_scan",
        }
    }

    const fn separate(self) -> bool {
        matches!(self, Self::GenericIcrc3)
    }
}

#[derive(CandidType, Deserialize)]
struct DebugInitArgs {
    observed_ledger: Principal,
    icp_ledger: Principal,
    cmc_canister: Principal,
    historian_canister: Principal,
    faucet_canister: Principal,
    surplus_canister: Option<Principal>,
    sns_root: Option<Principal>,
}

#[derive(CandidType, Deserialize)]
struct DebugProfile {
    symbol: String,
    decimals: u8,
    supports_2xfer: bool,
}

#[derive(Clone, Copy, CandidType, Deserialize)]
enum DebugBatchKind {
    Specific,
    Irrelevant,
}

#[derive(CandidType, Deserialize)]
struct DebugAppendBatch {
    kind: DebugBatchKind,
    start: u64,
    count: u64,
}

#[derive(CandidType, Deserialize)]
struct DebugSeedSubscribers {
    kind: String,
    start: u64,
    count: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Eq, PartialEq)]
struct Account {
    owner: Principal,
    subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize)]
struct Append {
    from: Account,
    to: Account,
    amount_e8s: u64,
    icrc1_memo: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize)]
struct SetRoute {
    destination_canister_id: Principal,
    memo: Vec<u8>,
    total_e8s: u64,
}

#[derive(CandidType, Deserialize)]
struct SetBalance {
    account: Account,
    e8s: u64,
}

#[derive(CandidType, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
enum WireMode {
    ReserveProtection,
    Economy,
    Standard,
    Fast,
    VeryFast,
    Continuous,
}

#[derive(CandidType, serde::Serialize)]
#[allow(dead_code)]
enum CallbackTarget {
    #[serde(rename = "subaccount")]
    Subaccount(u64),
    #[serde(rename = "neuron_nonce")]
    NeuronNonce(u64),
}

#[derive(CandidType, serde::Serialize)]
struct CallbackMatch {
    subject: Principal,
    first_block_index: u64,
    target: CallbackTarget,
    max_amount: Nat,
}

#[derive(CandidType)]
struct CallbackNotification {
    global_activity: bool,
    matches: Vec<CallbackMatch>,
}

impl From<WireMode> for Mode {
    fn from(value: WireMode) -> Self {
        match value {
            WireMode::ReserveProtection => Self::ReserveProtection,
            WireMode::Economy => Self::Economy,
            WireMode::Standard => Self::Standard,
            WireMode::Fast => Self::Fast,
            WireMode::VeryFast => Self::VeryFast,
            WireMode::Continuous => Self::Continuous,
        }
    }
}

#[derive(CandidType, Deserialize, Debug)]
struct DebugState {
    admission_bootstrapped: bool,
    admission_next_block: u64,
    observed_bootstrapped: bool,
    observed_next_block: u64,
    polling_mode: WireMode,
    subscriptions: u64,
    global_subscriptions: u64,
    cmc_state: String,
    surplus_policy: String,
    surplus_destination: Option<Principal>,
    pricing: Pricing,
}

#[derive(CandidType, Deserialize, Debug)]
struct Pricing {
    initialized: bool,
    current: Price,
    current_effective_at: u64,
    next: Option<Price>,
    next_effective_at: u64,
    next_freeze_at: u64,
    observed_floor_xdr_permyriad: u64,
    floor_observed_at: u64,
    latest_xdr_permyriad: u64,
    latest_observed_at: u64,
    next_carried_forward_due_to_stale_rate: bool,
}

#[derive(CandidType, Deserialize, Debug)]
struct Price {
    account_icp: u64,
    range_icp: u64,
    global_icp: u64,
}

struct Fixture {
    pic: PocketIc,
    architecture: Architecture,
    ledger: Principal,
    observed_ledger: Principal,
    historian: Principal,
    cmc: Principal,
    subscriber: Principal,
    subscriber_second: Principal,
    backend: Principal,
    faucet: Principal,
    initial_backend_cycles: u128,
}

impl Fixture {
    fn new(architecture: Architecture, backend_cycles: u128) -> Result<Self> {
        let pic = PocketIcBuilder::new().with_application_subnet().build();
        let ledger = pic.create_canister();
        let observed_ledger = if architecture.separate() {
            pic.create_canister()
        } else {
            ledger
        };
        let historian = pic.create_canister();
        let cmc = pic.create_canister();
        let subscriber = pic.create_canister();
        let subscriber_second = pic.create_canister();
        let backend = pic
            .create_canister_with_params(
                None,
                CreateCanisterParams {
                    cycles: Some(backend_cycles),
                    settings: None,
                    placement: None,
                },
            )
            .map_err(|error| anyhow!("create Event Horizon canister: {error}"))?;

        for id in [
            ledger,
            observed_ledger,
            historian,
            cmc,
            subscriber,
            subscriber_second,
        ] {
            pic.add_cycles(id, 200 * TRILLION);
        }
        pic.install_canister(
            ledger,
            wasm(&LEDGER_WASM, "mock-icp-ledger", None)?,
            vec![],
            None,
        );
        if architecture.separate() {
            pic.install_canister(
                observed_ledger,
                wasm(
                    &ICRC3_LEDGER_WASM,
                    "mock-icrc3-ledger",
                    Some("generic_icrc3"),
                )?,
                vec![],
                None,
            );
            update::<_, ()>(
                &pic,
                observed_ledger,
                "debug_set_profile",
                DebugProfile {
                    symbol: "CAL".into(),
                    decimals: 8,
                    supports_2xfer: true,
                },
            )?;
        }
        pic.install_canister(
            historian,
            wasm(&HISTORIAN_WASM, "mock-historian", None)?,
            vec![],
            None,
        );
        pic.install_canister(cmc, wasm(&CMC_WASM, "mock-cmc", None)?, vec![], None);
        pic.install_canister(
            subscriber,
            wasm(&SUBSCRIBER_WASM, "mock-subscriber", None)?,
            vec![],
            None,
        );
        pic.install_canister(
            subscriber_second,
            wasm(&SUBSCRIBER_WASM, "mock-subscriber", None)?,
            vec![],
            None,
        );
        let faucet = Principal::from_slice(&[42]);
        pic.install_canister(
            backend,
            wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?,
            encode_one(DebugInitArgs {
                observed_ledger,
                icp_ledger: ledger,
                cmc_canister: cmc,
                historian_canister: historian,
                faucet_canister: faucet,
                surplus_canister: None,
                sns_root: None,
            })?,
            None,
        );
        let initial_backend_cycles = pic.cycle_balance(backend);
        Ok(Self {
            pic,
            architecture,
            ledger,
            observed_ledger,
            historian,
            cmc,
            subscriber,
            subscriber_second,
            backend,
            faucet,
            initial_backend_cycles,
        })
    }

    fn state(&self) -> Result<DebugState> {
        query(&self.pic, self.backend, "debug_state", ())
    }

    fn poll(&self) -> Result<()> {
        update(&self.pic, self.backend, "debug_poll_once", ())
    }

    fn price_once(&self) -> Result<()> {
        update(&self.pic, self.backend, "debug_pricing_once", ())
    }

    fn start_all_schedulers(&self) -> Result<()> {
        update(&self.pic, self.backend, "debug_start_schedulers", ())
    }

    fn refresh_mode(&self) -> Result<Mode> {
        self.start_all_schedulers()?;
        Ok(self.state()?.polling_mode.into())
    }

    fn set_scheduler_balance(&self, value: Option<u128>) -> Result<()> {
        update(
            &self.pic,
            self.backend,
            "debug_set_scheduler_liquid_cycles_override",
            value,
        )
    }

    fn scheduler_counts(&self, lane: &str) -> Result<(u64, u64)> {
        let values: Vec<u64> = query(
            &self.pic,
            self.backend,
            "debug_scheduler_lane_counts",
            lane.to_string(),
        )?;
        match values.as_slice() {
            [starts, busy] => Ok((*starts, *busy)),
            _ => bail!("unexpected scheduler counter shape: {values:?}"),
        }
    }

    fn scheduler_state(&self, lane: &str) -> Result<(bool, bool)> {
        let values: Vec<bool> = query(
            &self.pic,
            self.backend,
            "debug_scheduler_lane_state",
            lane.to_string(),
        )?;
        match values.as_slice() {
            [running, scheduled] => Ok((*running, *scheduled)),
            _ => bail!("unexpected scheduler state shape: {values:?}"),
        }
    }

    fn wait_for_completed_lane(
        &self,
        lane: &str,
        previous_starts: u64,
        max_ticks: u64,
    ) -> Result<u64> {
        for ticks in 1..=max_ticks {
            self.pic.tick();
            let (starts, _) = self.scheduler_counts(lane)?;
            let (running, _) = self.scheduler_state(lane)?;
            if starts > previous_starts && !running {
                return Ok(ticks);
            }
        }
        bail!(
            "{lane} lane did not complete within {max_ticks} ticks; counts={:?} lane={:?} state={:?}",
            self.scheduler_counts(lane)?,
            self.scheduler_state(lane)?,
            self.state()?
        )
    }

    fn wait_for_completed_poll(&self, previous_starts: u64, max_ticks: u64) -> Result<u64> {
        self.wait_for_completed_lane("poll", previous_starts, max_ticks)
    }

    fn warm_all_schedulers(&self, max_ticks: u64) -> Result<()> {
        let before = [
            self.scheduler_counts("poll")?.0,
            self.scheduler_counts("funding")?.0,
            self.scheduler_counts("pricing")?.0,
        ];
        self.start_all_schedulers()?;
        for _ in 0..max_ticks {
            self.pic.tick();
            let mut complete = true;
            for (index, lane) in ["poll", "funding", "pricing"].into_iter().enumerate() {
                let (starts, _) = self.scheduler_counts(lane)?;
                let (running, _) = self.scheduler_state(lane)?;
                complete &= starts > before[index] && !running;
            }
            let state = self.state()?;
            if complete && state.pricing.initialized && state.cmc_state.contains("Idle") {
                return Ok(());
            }
        }
        bail!("scheduler warm-up did not complete within {max_ticks} ticks")
    }

    fn ledger_calls(&self) -> Result<u64> {
        let legacy: u64 = query(
            &self.pic,
            self.backend,
            "debug_legacy_query_blocks_calls",
            (),
        )?;
        let icrc3: u64 = query(&self.pic, self.backend, "debug_icrc3_get_blocks_calls", ())?;
        Ok(legacy + icrc3)
    }

    fn poke_counts(&self) -> Result<(u64, u64)> {
        let values: Vec<u64> = query(&self.pic, self.backend, "debug_poke_counts", ())?;
        match values.as_slice() {
            [attempts, accepted] => Ok((*attempts, *accepted)),
            _ => bail!("unexpected poke counter shape: {values:?}"),
        }
    }

    fn seed_subscribers(&self, kind: &str, count: u64) -> Result<()> {
        for start in (0..count).step_by(128) {
            update::<_, ()>(
                &self.pic,
                self.backend,
                "debug_seed_subscribers",
                DebugSeedSubscribers {
                    kind: kind.into(),
                    start,
                    count: (count - start).min(128),
                },
            )?;
        }
        Ok(())
    }

    fn append_batch(&self, kind: DebugBatchKind, count: u64) -> Result<()> {
        let target = self.observed_ledger;
        for start in (0..count).step_by(128) {
            update::<_, u64>(
                &self.pic,
                target,
                "debug_append_batch",
                DebugAppendBatch {
                    kind,
                    start,
                    count: (count - start).min(128),
                },
            )?;
        }
        Ok(())
    }

    fn append_mixed_irrelevant(&self, count: u64) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        update::<_, u64>(
            &self.pic,
            self.observed_ledger,
            "debug_append_other",
            "3approve".to_string(),
        )?;
        if count > 1 {
            self.append_batch(DebugBatchKind::Irrelevant, count - 1)?;
        }
        Ok(())
    }

    fn set_first_local_block(&self, index: u64) -> Result<()> {
        update(
            &self.pic,
            self.observed_ledger,
            "debug_set_first_local_block",
            index,
        )
    }

    fn append_transfer_to(
        &self,
        ledger: Principal,
        subscriber: Principal,
        subaccount: u64,
    ) -> Result<u64> {
        update(
            &self.pic,
            ledger,
            "debug_append_transfer",
            Append {
                from: account(Principal::from_slice(&[9]), 999),
                to: account(subscriber, subaccount),
                amount_e8s: 1_000_000,
                icrc1_memo: Some(vec![0xca, 0x1b]),
            },
        )
    }

    fn set_route(&self, memo: Vec<u8>, total_e8s: u64) -> Result<()> {
        update(
            &self.pic,
            self.historian,
            "debug_set_route",
            SetRoute {
                destination_canister_id: self.backend,
                memo,
                total_e8s,
            },
        )
    }

    fn append_admission(
        &self,
        subscriber: Principal,
        declaration: &str,
        total_e8s: u64,
    ) -> Result<()> {
        let compact = subscriber.to_text().replace('-', "");
        let memo = format!("{compact}{declaration}").into_bytes();
        self.set_route(memo.clone(), total_e8s)?;
        update::<_, u64>(
            &self.pic,
            self.ledger,
            "debug_append_transfer",
            Append {
                from: account(self.faucet, 0),
                to: account(self.backend, 0),
                amount_e8s: 10_000_000,
                icrc1_memo: Some(memo),
            },
        )?;
        self.poll()
    }

    fn admit_global(&self, subscriber: Principal) -> Result<()> {
        self.append_admission(subscriber, "", 1_000 * ICP_E8S)
    }

    fn admit_account(&self, subscriber: Principal, subaccount: u64) -> Result<()> {
        self.append_admission(subscriber, &format!(".{subaccount}"), 10 * ICP_E8S)
    }

    fn admit_range(&self, subscriber: Principal, start: u64, end: u64) -> Result<()> {
        self.append_admission(subscriber, &format!(".{start}-{end}"), 20 * ICP_E8S)
    }

    fn subscriber_pokes(&self, second: bool) -> Result<u64> {
        query(
            &self.pic,
            if second {
                self.subscriber_second
            } else {
                self.subscriber
            },
            "debug_pokes",
            (),
        )
    }

    fn wait_for_subscriber_pokes(
        &self,
        second: bool,
        expected: u64,
        max_ticks: u64,
    ) -> Result<u64> {
        for ticks in 0..=max_ticks {
            if self.subscriber_pokes(second)? >= expected {
                return Ok(ticks);
            }
            if ticks < max_ticks {
                self.pic.tick();
            }
        }
        bail!(
            "subscriber callback did not reach {expected} within {max_ticks} ticks; observed={}",
            self.subscriber_pokes(second)?
        )
    }

    fn settle_cycles(&self, max_ticks: u64) -> Result<()> {
        let mut stable = 0;
        let mut previous = self.pic.cycle_balance(self.backend);
        for _ in 0..max_ticks {
            self.pic.tick();
            let current = self.pic.cycle_balance(self.backend);
            if current == previous {
                stable += 1;
                if stable >= 3 {
                    return Ok(());
                }
            } else {
                stable = 0;
                previous = current;
            }
        }
        bail!("backend cycle balance did not settle within {max_ticks} ticks")
    }

    fn set_raw_icp_balance(&self, e8s: u64) -> Result<()> {
        update(
            &self.pic,
            self.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: self.backend,
                    subaccount: None,
                },
                e8s,
            },
        )
    }

    fn accepted_topups(&self) -> Result<u64> {
        query(
            &self.pic,
            self.ledger,
            "debug_accepted_legacy_transfers_with_memo",
            1_347_768_404_u64,
        )
    }

    fn cmc_calls(&self) -> Result<u64> {
        query(&self.pic, self.cmc, "debug_calls", ())
    }
}

pub fn run(profile: &str, config: &RunConfig) -> Result<MeasurementSet> {
    if !matches!(profile, "smoke" | "full") {
        bail!("unknown measurement profile `{profile}`")
    }
    if config.limits.maximum_ticks_per_wait == 0
        || config.limits.maximum_completed_polls == 0
        || config.limits.maximum_processed_blocks == 0
        || config.limits.maximum_wall_seconds == 0
    {
        bail!("all execution limits must be positive")
    }
    fs::create_dir_all(&config.output)
        .with_context(|| format!("create {}", config.output.display()))?;
    let started = Instant::now();
    let mut scenarios = Vec::new();
    let modes: &[Mode] = if profile == "smoke" {
        &[Mode::Economy, Mode::Continuous]
    } else {
        &Mode::ACTIVE
    };
    for &mode in modes {
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.push(measure_baseline(
            mode,
            if profile == "smoke" { 2 } else { 3 },
            config,
        )?);
    }
    if profile == "full" {
        for trial in 1..=3 {
            ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
            let mut measurement = measure_baseline(Mode::VeryFast, 1, config)?;
            measurement.scenario_id = format!("repeatability_very_fast_fresh_fixture_{trial}");
            measurement.experiment = "A_repeatability".into();
            measurement.notes.push(format!(
                "Fresh equivalent PocketIC fixture repeat {trial} of 3."
            ));
            scenarios.push(measurement);
        }
    }
    scenarios.push(measure_reserve_recovery(config)?);
    if profile == "full" {
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.push(measure_integrated_scheduler(config)?);
    }

    let traffic = if profile == "smoke" {
        vec![
            ("light_1_block_per_second", 10_u64, 2_u64),
            ("burst_512_blocks", 512_u64, 1_u64),
        ]
    } else {
        vec![
            ("idle_0_blocks_per_second", 0, 3),
            ("light_1_block_per_second", 10, 3),
            ("moderate_5_blocks_per_second", 50, 3),
            ("heavy_20_blocks_per_second", 200, 3),
            ("burst_1024_blocks", 1_024, 1),
            ("burst_4096_blocks", 4_096, 1),
        ]
    };
    let architectures: &[Architecture] = &[Architecture::CanonicalIcp, Architecture::GenericIcrc3];
    for &architecture in architectures {
        for &(name, blocks, intervals) in &traffic {
            ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
            scenarios.push(measure_traffic(
                architecture,
                name,
                blocks,
                intervals,
                config,
            )?);
        }
    }

    let fanouts: &[u64] = if profile == "smoke" {
        &[0, 10]
    } else {
        &[0, 1, 10, 100, 256, 1_024]
    };
    for &count in fanouts {
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.push(measure_fanout("global", count, config)?);
    }
    if profile == "full" {
        for architecture in [Architecture::CanonicalIcp, Architecture::GenericIcrc3] {
            ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
            scenarios.push(measure_archive_prefix(architecture, config)?);
        }
        for count in [1, 100, 256, 1_024] {
            ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
            scenarios.push(measure_fanout("specific_matching", count, config)?);
        }
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.push(measure_specific_no_match(config)?);
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.extend(measure_real_subscriber_shapes(config)?);
        for mode in Mode::ACTIVE {
            ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
            scenarios.push(measure_latency(mode, false, config)?);
        }
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.push(measure_latency(Mode::VeryFast, true, config)?);
        ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
        scenarios.extend(measure_large_balances(config)?);
    }

    ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;
    let threshold_observations = measure_thresholds(config)?;
    if threshold_observations.iter().any(|item| !item.agrees) {
        bail!("PocketIC threshold observations diverged from the production next_mode function")
    }
    if scenarios.iter().any(|item| !item.complete) {
        bail!("one or more calibration scenarios did not complete")
    }
    ensure_wall_budget(started, config.limits.maximum_wall_seconds)?;

    let debug_wasm = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
    let set = MeasurementSet {
        schema_version: 1,
        provenance: Provenance {
            git_commit: git(&["rev-parse", "HEAD"] )?,
            working_tree_dirty: !git_status_clean()?,
            debug_wasm_sha256: hex::encode(Sha256::digest(&debug_wasm)),
            pocketic_version: "15.0.0".into(),
            subnet_topology: "one PocketIC application subnet".into(),
            cycles_cost_schedule: "PocketIC normal cost schedule".into(),
            seed: config.seed,
            profile: profile.into(),
            generated_at_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
            limits: config.limits.clone(),
        },
        scenarios,
        threshold_observations,
        limitations: vec![
            "PocketIC removes consensus/networking and is not a mainnet latency distribution.".into(),
            "Mock Ledgers preserve the interfaces and representative block fields used by Event Horizon, but not mainnet response encoding size, replica load, or archive subnet placement.".into(),
            "Synthetic fan-out principals measure Event Horizon dispatch attempts and accepted one-way calls; only the dedicated real-subscriber cases prove receiver execution.".into(),
            "Continuous has zero added timer delay; PocketIC rounds can execute successive polls at unchanged simulated time, so no polls-per-second claim is derived from host wall time.".into(),
        ],
    };
    write_measurements(&set, &config.output)?;
    Ok(set)
}

fn measure_archive_prefix(architecture: Architecture, config: &RunConfig) -> Result<Measurement> {
    let fixture = Fixture::new(architecture, 200 * TRILLION)?;
    fixture.poll()?;
    fixture.append_batch(DebugBatchKind::Irrelevant, 768)?;
    fixture.append_mixed_irrelevant(1)?;
    fixture.set_first_local_block(768)?;
    let cursor_before = fixture.state()?.observed_next_block;
    let calls_before = fixture.ledger_calls()?;
    let cycles_before = fixture.pic.cycle_balance(fixture.backend);
    fixture.poll()?;
    fixture.settle_cycles(config.limits.maximum_ticks_per_wait)?;
    let cycles_after = fixture.pic.cycle_balance(fixture.backend);
    let cursor_after = fixture.state()?.observed_next_block;
    if cursor_after - cursor_before != 769 {
        bail!(
            "archive-prefix scenario advanced {} blocks instead of 769",
            cursor_after - cursor_before
        )
    }
    let calls = fixture.ledger_calls()? - calls_before;
    let mut result = base_measurement(
        format!("archive_prefix_{}", architecture.id()),
        "B_ledger_volume",
        architecture,
        Some(Mode::Continuous),
        "archive_prefix_768_live_tail_1",
        fixture.initial_backend_cycles,
    );
    result.backend_cycles_before = cycles_before;
    result.backend_cycles_after = cycles_after;
    result.net_cycles_consumed = cycles_before.saturating_sub(cycles_after);
    result.per_poll_cycles = vec![result.net_cycles_consumed];
    fill_cycle_summary(&mut result);
    result.injected_blocks = 769;
    result.processed_blocks = 1;
    result.completed_polls = 1;
    result.completed_ledger_calls = calls;
    result.ledger_pages = calls;
    result.cursor_before = cursor_before;
    result.cursor_after = cursor_after;
    result.maximum_ledger_lag = 769;
    result.average_ledger_lag = 385;
    result.catch_up_time_seconds = Some(0);
    result.notes.push(
        "The reader validated and skipped a 768-block archive prefix without following archive callbacks, then decoded the one live-tail block; cursor advancement therefore exceeds processed_blocks.".into(),
    );
    Ok(result)
}

fn measure_baseline(mode: Mode, repeats: u64, config: &RunConfig) -> Result<Measurement> {
    ensure_poll_budget(repeats, config)?;
    let fixture = Fixture::new(Architecture::CanonicalIcp, balance_for_mode(mode))?;
    fixture.poll()?;
    let initial_cursor = fixture.state()?.observed_next_block;
    fixture.warm_all_schedulers(config.limits.maximum_ticks_per_wait)?;
    let mode_observed: Mode = fixture.state()?.polling_mode.into();
    if mode_observed != mode {
        bail!("baseline requested {mode:?}, observed {mode_observed:?}")
    }
    let before = fixture.pic.cycle_balance(fixture.backend);
    let calls_before = fixture.ledger_calls()?;
    let time_before = now_seconds(&fixture.pic);
    let mut costs = Vec::new();
    let mut previous_starts = fixture.scheduler_counts("poll")?.0;
    for _ in 0..repeats {
        let funding_starts = fixture.scheduler_counts("funding")?.0;
        let delay = mode.delay_seconds(0).expect("active mode");
        if delay > 0 {
            fixture.pic.advance_time(Duration::from_secs(delay));
        }
        let poll_before = fixture.pic.cycle_balance(fixture.backend);
        fixture.wait_for_completed_poll(previous_starts, config.limits.maximum_ticks_per_wait)?;
        previous_starts = fixture.scheduler_counts("poll")?.0;
        if mode == Mode::Economy {
            fixture.wait_for_completed_lane(
                "funding",
                funding_starts,
                config.limits.maximum_ticks_per_wait,
            )?;
        }
        let poll_after = fixture.pic.cycle_balance(fixture.backend);
        costs.push(poll_before.saturating_sub(poll_after));
    }
    let after = fixture.pic.cycle_balance(fixture.backend);
    let calls = fixture.ledger_calls()?.saturating_sub(calls_before);
    let mut result = base_measurement(
        format!("baseline_{}", mode_id(mode)),
        "A_baseline_polling",
        fixture.architecture,
        Some(mode),
        "idle",
        fixture.initial_backend_cycles,
    );
    result.backend_cycles_before = before;
    result.backend_cycles_after = after;
    result.net_cycles_consumed = before.saturating_sub(after);
    result.per_poll_cycles = costs;
    fill_cycle_summary(&mut result);
    result.initial_ledger_blocks = initial_cursor;
    result.cursor_before = initial_cursor;
    result.cursor_after = fixture.state()?.observed_next_block;
    result.processed_blocks = result.cursor_after.saturating_sub(result.cursor_before);
    result.completed_polls = repeats;
    result.completed_ledger_calls = calls;
    result.ledger_pages = calls;
    result.simulated_time_elapsed_seconds = now_seconds(&fixture.pic) - time_before;
    result.scheduler_driven = true;
    result.maintenance_included = mode == Mode::Economy;
    result.notes.push("Installation, cursor bootstrap, and one complete all-lane scheduler warm-up occurred before the measurement window.".into());
    if mode == Mode::Economy {
        result.notes.push("Economy's one-hour poll delay equals the funding-maintenance interval, so these timer-driven samples include the funding lane and are excluded from isolated empty-poll calibration.".into());
    } else {
        result.notes.push("The bounded window ended before the next funding or pricing maintenance timer, so its cycle deltas isolate ordinary timer-driven polling after warm-up.".into());
    }
    Ok(result)
}

fn measure_reserve_recovery(config: &RunConfig) -> Result<Measurement> {
    let fixture = Fixture::new(Architecture::CanonicalIcp, TRILLION)?;
    fixture.poll()?;
    fixture.set_raw_icp_balance(ICP_E8S)?;
    let cursor = fixture.state()?.observed_next_block;
    let calls = fixture.ledger_calls()?;
    let before = fixture.pic.cycle_balance(fixture.backend);
    fixture.start_all_schedulers()?;
    let mut cap = true;
    for _ in 0..config.limits.maximum_ticks_per_wait {
        fixture.pic.tick();
        if fixture.accepted_topups()? == 1
            && fixture.cmc_calls()? > 0
            && fixture.state()?.cmc_state.contains("Idle")
        {
            cap = false;
            break;
        }
    }
    let after = fixture.pic.cycle_balance(fixture.backend);
    let state = fixture.state()?;
    if cap {
        bail!("Reserve Protection funding lane did not complete before the tick cap")
    }
    if Mode::from(state.polling_mode) != Mode::ReserveProtection
        || state.observed_next_block != cursor
        || fixture.ledger_calls()? != calls
    {
        bail!("Reserve Protection allowed ordinary polling or left its mode")
    }
    let balance_before_topup = fixture.pic.cycle_balance(fixture.backend);
    fixture.pic.add_cycles(fixture.backend, 20 * TRILLION);
    let recovered = fixture.refresh_mode()?;
    if recovered == Mode::ReserveProtection {
        bail!("successful synthetic CMC side effect did not permit cadence recovery")
    }
    let mut result = base_measurement(
        "reserve_protection_funding_recovery".into(),
        "A_reserve_protection",
        fixture.architecture,
        Some(Mode::ReserveProtection),
        "idle_with_raw_icp_receipt",
        fixture.initial_backend_cycles,
    );
    result.backend_cycles_before = before;
    result.backend_cycles_after = after;
    result.net_cycles_consumed = before.saturating_sub(after);
    result.cursor_before = cursor;
    result.cursor_after = state.observed_next_block;
    result.completed_polls = 0;
    result.completed_ledger_calls = 0;
    result.maintenance_included = true;
    result.scheduler_driven = true;
    result.notes.extend([
        "Ordinary polling stayed suspended while funding and pricing scheduler lanes remained live.".into(),
        format!("Funding transfer was accepted; PocketIC cannot mint in the mock CMC, so the real CMC side effect was represented by adding 20 T synthetic cycles after the measurement window (balance before add: {balance_before_topup})."),
        format!("Cadence after the synthetic mint side effect: {}.", recovered.label()),
    ]);
    Ok(result)
}

fn measure_integrated_scheduler(config: &RunConfig) -> Result<Measurement> {
    let fixture = Fixture::new(Architecture::CanonicalIcp, 11 * TRILLION)?;
    fixture.poll()?;
    let cursor = fixture.state()?.observed_next_block;
    fixture.start_all_schedulers()?;
    let cycles_before = fixture.pic.cycle_balance(fixture.backend);
    let calls_before = fixture.ledger_calls()?;
    let starts_before = fixture.scheduler_counts("poll")?.0;
    fixture.wait_for_completed_poll(starts_before, config.limits.maximum_ticks_per_wait)?;
    let mut maintenance_complete = false;
    for _ in 0..config.limits.maximum_ticks_per_wait {
        fixture.pic.tick();
        let state = fixture.state()?;
        if state.pricing.initialized && state.cmc_state.contains("Idle") {
            maintenance_complete = true;
            break;
        }
    }
    if !maintenance_complete {
        bail!("integrated Fast scheduler maintenance did not complete before the tick cap")
    }
    let cycles_after = fixture.pic.cycle_balance(fixture.backend);
    let calls = fixture.ledger_calls()? - calls_before;
    let mut result = base_measurement(
        "integrated_fast_all_scheduler_lanes".into(),
        "A_integrated_scheduler",
        fixture.architecture,
        Some(Mode::Fast),
        "idle",
        fixture.initial_backend_cycles,
    );
    result.backend_cycles_before = cycles_before;
    result.backend_cycles_after = cycles_after;
    result.net_cycles_consumed = cycles_before.saturating_sub(cycles_after);
    result.per_poll_cycles = vec![result.net_cycles_consumed];
    fill_cycle_summary(&mut result);
    result.completed_polls = 1;
    result.completed_ledger_calls = calls;
    result.ledger_pages = calls;
    result.cursor_before = cursor;
    result.cursor_after = fixture.state()?.observed_next_block;
    result.maintenance_included = true;
    result.scheduler_driven = true;
    result.notes.push("One real Fast polling wrapper ran concurrently with the ordinary funding and pricing scheduler lanes; its cycle delta is intentionally integrated and is not used as the isolated empty-poll coefficient.".into());
    Ok(result)
}

fn measure_traffic(
    architecture: Architecture,
    profile: &str,
    blocks_per_interval: u64,
    intervals: u64,
    config: &RunConfig,
) -> Result<Measurement> {
    ensure_poll_budget(intervals, config)?;
    let total = blocks_per_interval.saturating_mul(intervals);
    if total > config.limits.maximum_processed_blocks {
        bail!(
            "{profile} requests {total} blocks, over configured cap {}",
            config.limits.maximum_processed_blocks
        )
    }
    let fixture = Fixture::new(architecture, 30 * TRILLION)?;
    fixture.poll()?;
    fixture.warm_all_schedulers(config.limits.maximum_ticks_per_wait)?;
    let mut starts = fixture.scheduler_counts("poll")?.0;
    let cursor_before = fixture.state()?.observed_next_block;
    let calls_before = fixture.ledger_calls()?;
    let cycles_before = fixture.pic.cycle_balance(fixture.backend);
    let time_before = now_seconds(&fixture.pic);
    let mut costs = Vec::new();
    let mut max_lag = 0_u64;
    let mut lag_sum = 0_u128;
    let mut lag_samples = 0_u128;
    for _ in 0..intervals {
        fixture.append_mixed_irrelevant(blocks_per_interval)?;
        max_lag = max_lag.max(blocks_per_interval);
        lag_sum += u128::from(blocks_per_interval);
        lag_samples += 1;
        let before_poll = fixture.pic.cycle_balance(fixture.backend);
        fixture.pic.advance_time(Duration::from_secs(10));
        fixture.wait_for_completed_poll(starts, config.limits.maximum_ticks_per_wait)?;
        starts = fixture.scheduler_counts("poll")?.0;
        costs.push(before_poll.saturating_sub(fixture.pic.cycle_balance(fixture.backend)));
        lag_samples += 1;
    }
    let cycles_after = fixture.pic.cycle_balance(fixture.backend);
    let cursor_after = fixture.state()?.observed_next_block;
    if cursor_after.saturating_sub(cursor_before) != total {
        bail!(
            "{profile} processed {} of {total} blocks",
            cursor_after.saturating_sub(cursor_before)
        )
    }
    let calls = fixture.ledger_calls()?.saturating_sub(calls_before);
    let mut result = base_measurement(
        format!("traffic_{}_{}", architecture.id(), profile),
        "B_ledger_volume",
        architecture,
        Some(Mode::VeryFast),
        profile,
        fixture.initial_backend_cycles,
    );
    result.backend_cycles_before = cycles_before;
    result.backend_cycles_after = cycles_after;
    result.net_cycles_consumed = cycles_before.saturating_sub(cycles_after);
    result.per_poll_cycles = costs;
    fill_cycle_summary(&mut result);
    result.injected_blocks = total;
    result.processed_blocks = total;
    result.completed_polls = intervals;
    result.completed_ledger_calls = calls;
    result.ledger_pages = calls;
    result.cursor_before = cursor_before;
    result.cursor_after = cursor_after;
    result.maximum_ledger_lag = max_lag;
    result.average_ledger_lag = if lag_samples == 0 {
        0
    } else {
        u64::try_from(lag_sum / lag_samples).unwrap_or(u64::MAX)
    };
    result.catch_up_time_seconds = Some(if total == 0 { 0 } else { 10 });
    result.simulated_time_elapsed_seconds = now_seconds(&fixture.pic) - time_before;
    result.scheduler_driven = true;
    result.notes.push(
        "Each sustained interval appends activity, advances simulated time by the real Very Fast delay, and waits for the asynchronous poll to complete.".into(),
    );
    if total > 20 {
        result.notes.push(
            "The bounded bulk fixture uses full transfer records plus a representative non-transfer block; it does not reproduce mainnet response byte sizes.".into(),
        );
    }
    Ok(result)
}

fn measure_fanout(kind: &str, count: u64, config: &RunConfig) -> Result<Measurement> {
    let fixture = Fixture::new(Architecture::GenericIcrc3, 2_000 * TRILLION)?;
    fixture.poll()?;
    let seed_kind = if kind == "global" {
        "global"
    } else {
        "specific"
    };
    fixture.seed_subscribers(seed_kind, count)?;
    let blocks = if kind == "global" { 1 } else { count };
    if kind == "global" {
        fixture.append_mixed_irrelevant(1)?;
    } else {
        fixture.append_batch(DebugBatchKind::Specific, count)?;
    }
    let cursor_before = fixture.state()?.observed_next_block;
    let calls_before = fixture.ledger_calls()?;
    let pokes_before = fixture.poke_counts()?;
    let cycles_before = fixture.pic.cycle_balance(fixture.backend);
    fixture.poll()?;
    fixture.settle_cycles(config.limits.maximum_ticks_per_wait)?;
    let cycles_after = fixture.pic.cycle_balance(fixture.backend);
    let pokes_after = fixture.poke_counts()?;
    let cursor_after = fixture.state()?.observed_next_block;
    let attempts = pokes_after.0.saturating_sub(pokes_before.0);
    if attempts != count {
        bail!("{kind} fan-out expected {count} attempts, observed {attempts}")
    }
    let mut result = base_measurement(
        format!("fanout_{kind}_{count}"),
        "C_notification_fanout",
        fixture.architecture,
        Some(Mode::Continuous),
        if kind == "global" {
            "one_irrelevant_transfer"
        } else {
            "one_matching_transfer_per_subscriber"
        },
        fixture.initial_backend_cycles,
    );
    result.subscriber_kind = kind.into();
    result.subscriber_count = count;
    result.backend_cycles_before = cycles_before;
    result.backend_cycles_after = cycles_after;
    result.net_cycles_consumed = cycles_before.saturating_sub(cycles_after);
    result.per_poll_cycles = vec![result.net_cycles_consumed];
    fill_cycle_summary(&mut result);
    result.injected_blocks = blocks;
    result.processed_blocks = cursor_after.saturating_sub(cursor_before);
    result.completed_polls = 1;
    result.completed_ledger_calls = fixture.ledger_calls()?.saturating_sub(calls_before);
    result.ledger_pages = result.completed_ledger_calls;
    result.cursor_before = cursor_before;
    result.cursor_after = cursor_after;
    result.poke_attempts = attempts;
    result.accepted_oneway_calls = pokes_after.1.saturating_sub(pokes_before.1);
    result.callback_payload_bytes = if count == 0 {
        None
    } else {
        let mut bytes = [0u8; 9];
        bytes[0] = 0x7f;
        bytes[1..].copy_from_slice(&(count - 1).to_be_bytes());
        Some(
            encode_one(CallbackNotification {
                global_activity: kind == "global",
                matches: if kind == "global" {
                    vec![]
                } else {
                    vec![CallbackMatch {
                        subject: Principal::from_slice(&bytes),
                        target: CallbackTarget::Subaccount(0),
                        first_block_index: cursor_before + count - 1,
                        max_amount: Nat::from(count),
                    }]
                },
            })?
            .len() as u64,
        )
    };
    result.notes.push(
        "Synthetic principals isolate backend dispatch pressure; accepted one-way calls are not recipient execution evidence. The recorded callback payload is the exact global-only notification or the largest specific-only notification with one match in the scenario. Balance was sampled after rejection/refund traffic settled.".into(),
    );
    Ok(result)
}

fn measure_specific_no_match(config: &RunConfig) -> Result<Measurement> {
    let mut result = measure_fanout("global", 0, config)?;
    let fixture = Fixture::new(Architecture::GenericIcrc3, 200 * TRILLION)?;
    fixture.poll()?;
    fixture.seed_subscribers("specific", 100)?;
    fixture.append_mixed_irrelevant(1)?;
    let cursor_before = fixture.state()?.observed_next_block;
    let poke_before = fixture.poke_counts()?;
    let before = fixture.pic.cycle_balance(fixture.backend);
    let calls_before = fixture.ledger_calls()?;
    fixture.poll()?;
    fixture.settle_cycles(config.limits.maximum_ticks_per_wait)?;
    let after = fixture.pic.cycle_balance(fixture.backend);
    let poke_after = fixture.poke_counts()?;
    if poke_after != poke_before {
        bail!("irrelevant transfer notified specific subscribers")
    }
    result.scenario_id = "specific_100_no_relevant_transfer".into();
    result.subscriber_kind = "specific_no_match".into();
    result.subscriber_count = 100;
    result.backend_cycles_before = before;
    result.backend_cycles_after = after;
    result.net_cycles_consumed = before.saturating_sub(after);
    result.per_poll_cycles = vec![result.net_cycles_consumed];
    fill_cycle_summary(&mut result);
    result.injected_blocks = 1;
    result.processed_blocks = 1;
    result.completed_ledger_calls = fixture.ledger_calls()?.saturating_sub(calls_before);
    result.ledger_pages = result.completed_ledger_calls;
    result.cursor_before = cursor_before;
    result.cursor_after = fixture.state()?.observed_next_block;
    result.notes = vec![
        "One hundred specific subscriptions produced no dispatch for an irrelevant transfer."
            .into(),
    ];
    Ok(result)
}

fn measure_real_subscriber_shapes(config: &RunConfig) -> Result<Vec<Measurement>> {
    let mut out = Vec::new();

    let fixture = Fixture::new(Architecture::GenericIcrc3, 200 * TRILLION)?;
    fixture.price_once()?;
    fixture.poll()?;
    fixture.admit_global(fixture.subscriber)?;
    let before_pokes = fixture.subscriber_pokes(false)?;
    for index in 0..10 {
        fixture.append_transfer_to(fixture.observed_ledger, Principal::from_slice(&[8]), index)?;
    }
    let cursor_before = fixture.state()?.observed_next_block;
    let cycles_before = fixture.pic.cycle_balance(fixture.backend);
    let pokes_before = fixture.poke_counts()?;
    fixture.poll()?;
    fixture.wait_for_subscriber_pokes(
        false,
        before_pokes + 1,
        config.limits.maximum_ticks_per_wait,
    )?;
    let cycles_after = fixture.pic.cycle_balance(fixture.backend);
    let pokes_after = fixture.poke_counts()?;
    let mut coalesced = base_measurement(
        "real_global_coalesced_10_transactions".into(),
        "C_notification_fanout",
        fixture.architecture,
        Some(Mode::Continuous),
        "10_relevant_transactions",
        fixture.initial_backend_cycles,
    );
    coalesced.subscriber_kind = "global_real_receiver".into();
    coalesced.subscriber_count = 1;
    coalesced.backend_cycles_before = cycles_before;
    coalesced.backend_cycles_after = cycles_after;
    coalesced.net_cycles_consumed = cycles_before.saturating_sub(cycles_after);
    coalesced.per_poll_cycles = vec![coalesced.net_cycles_consumed];
    fill_cycle_summary(&mut coalesced);
    coalesced.injected_blocks = 10;
    coalesced.processed_blocks = fixture.state()?.observed_next_block - cursor_before;
    coalesced.completed_polls = 1;
    coalesced.poke_attempts = pokes_after.0 - pokes_before.0;
    coalesced.accepted_oneway_calls = pokes_after.1 - pokes_before.1;
    coalesced.observed_subscriber_callbacks = Some(1);
    coalesced
        .notes
        .push("Ten qualifying transfers were coalesced into one receiver-observed poke.".into());
    out.push(coalesced);

    let range = Fixture::new(Architecture::GenericIcrc3, 200 * TRILLION)?;
    range.price_once()?;
    range.poll()?;
    range.admit_range(range.subscriber, 0, 31)?;
    range.append_transfer_to(range.observed_ledger, range.subscriber, 17)?;
    let cursor_before = range.state()?.observed_next_block;
    let before = range.pic.cycle_balance(range.backend);
    let poke_counts = range.poke_counts()?;
    range.poll()?;
    range.wait_for_subscriber_pokes(false, 1, config.limits.maximum_ticks_per_wait)?;
    let after = range.pic.cycle_balance(range.backend);
    let after_counts = range.poke_counts()?;
    let mut expanded = base_measurement(
        "range_expanded_32_targets_one_match".into(),
        "C_notification_fanout",
        range.architecture,
        Some(Mode::Continuous),
        "one_matching_transfer",
        range.initial_backend_cycles,
    );
    expanded.subscriber_kind = "range_expanded_real_receiver".into();
    expanded.subscriber_count = 1;
    expanded.backend_cycles_before = before;
    expanded.backend_cycles_after = after;
    expanded.net_cycles_consumed = before.saturating_sub(after);
    expanded.per_poll_cycles = vec![expanded.net_cycles_consumed];
    fill_cycle_summary(&mut expanded);
    expanded.injected_blocks = 1;
    expanded.processed_blocks = range.state()?.observed_next_block - cursor_before;
    expanded.completed_polls = 1;
    expanded.poke_attempts = after_counts.0 - poke_counts.0;
    expanded.accepted_oneway_calls = after_counts.1 - poke_counts.1;
    expanded.observed_subscriber_callbacks = Some(1);
    expanded.notes.push("A 32-target range was expanded by the existing admission path; one target matched and one callback executed.".into());
    out.push(expanded);

    let mixed = Fixture::new(Architecture::GenericIcrc3, 200 * TRILLION)?;
    mixed.price_once()?;
    mixed.poll()?;
    mixed.admit_global(mixed.subscriber)?;
    mixed.admit_account(mixed.subscriber_second, 7)?;
    mixed.append_transfer_to(mixed.observed_ledger, mixed.subscriber_second, 7)?;
    let cursor_before = mixed.state()?.observed_next_block;
    let before = mixed.pic.cycle_balance(mixed.backend);
    let counts = mixed.poke_counts()?;
    mixed.poll()?;
    mixed.wait_for_subscriber_pokes(false, 1, config.limits.maximum_ticks_per_wait)?;
    mixed.wait_for_subscriber_pokes(true, 1, config.limits.maximum_ticks_per_wait)?;
    let after = mixed.pic.cycle_balance(mixed.backend);
    let after_counts = mixed.poke_counts()?;
    let mut mixed_result = base_measurement(
        "mixed_global_and_specific_real_receivers".into(),
        "C_notification_fanout",
        mixed.architecture,
        Some(Mode::Continuous),
        "one_matching_transfer",
        mixed.initial_backend_cycles,
    );
    mixed_result.subscriber_kind = "mixed_global_and_specific".into();
    mixed_result.subscriber_count = 2;
    mixed_result.backend_cycles_before = before;
    mixed_result.backend_cycles_after = after;
    mixed_result.net_cycles_consumed = before.saturating_sub(after);
    mixed_result.per_poll_cycles = vec![mixed_result.net_cycles_consumed];
    fill_cycle_summary(&mut mixed_result);
    mixed_result.injected_blocks = 1;
    mixed_result.processed_blocks = mixed.state()?.observed_next_block - cursor_before;
    mixed_result.completed_polls = 1;
    mixed_result.poke_attempts = after_counts.0 - counts.0;
    mixed_result.accepted_oneway_calls = after_counts.1 - counts.1;
    mixed_result.observed_subscriber_callbacks = Some(2);
    mixed_result
        .notes
        .push("One global and one matching specific subscriber each executed one callback.".into());
    out.push(mixed_result);
    Ok(out)
}

fn measure_latency(mode: Mode, burst: bool, config: &RunConfig) -> Result<Measurement> {
    let fixture = Fixture::new(Architecture::GenericIcrc3, balance_for_mode(mode))?;
    fixture.price_once()?;
    fixture.poll()?;
    fixture.admit_global(fixture.subscriber)?;
    fixture.warm_all_schedulers(config.limits.maximum_ticks_per_wait)?;
    let mut starts = fixture.scheduler_counts("poll")?.0;
    let delay = mode.delay_seconds(0).expect("active mode");
    let samples = if burst { 1 } else { 3 };
    ensure_poll_budget(samples, config)?;
    let mut latencies = Vec::new();
    let cursor_before = fixture.state()?.observed_next_block;
    let cycles_before = fixture.pic.cycle_balance(fixture.backend);
    let calls_before = fixture.ledger_calls()?;
    let poke_counts_before = fixture.poke_counts()?;
    let receiver_before = fixture.subscriber_pokes(false)?;
    let time_before = now_seconds(&fixture.pic);
    let blocks_each = if burst { 10 } else { 1 };
    for sample in 0..samples {
        fixture.append_mixed_irrelevant(blocks_each)?;
        let injected_at = now_seconds(&fixture.pic);
        let funding_starts = fixture.scheduler_counts("funding")?.0;
        if delay > 0 {
            fixture.pic.advance_time(Duration::from_secs(delay - 1));
            fixture.pic.tick();
            if fixture.subscriber_pokes(false)? != receiver_before + sample {
                bail!(
                    "{} latency callback arrived before its added delay",
                    mode.label()
                )
            }
            fixture.pic.advance_time(Duration::from_secs(1));
        }
        fixture.wait_for_completed_poll(starts, config.limits.maximum_ticks_per_wait)?;
        starts = fixture.scheduler_counts("poll")?.0;
        if mode == Mode::Economy {
            fixture.wait_for_completed_lane(
                "funding",
                funding_starts,
                config.limits.maximum_ticks_per_wait,
            )?;
        }
        fixture.wait_for_subscriber_pokes(
            false,
            receiver_before + sample + 1,
            config.limits.maximum_ticks_per_wait,
        )?;
        latencies.push(now_seconds(&fixture.pic).saturating_sub(injected_at));
    }
    let cycles_after = fixture.pic.cycle_balance(fixture.backend);
    let poke_counts_after = fixture.poke_counts()?;
    let cursor_after = fixture.state()?.observed_next_block;
    let mut result = base_measurement(
        format!(
            "latency_{}_{}",
            mode_id(mode),
            if burst { "burst_10" } else { "isolated" }
        ),
        "D_notification_latency",
        fixture.architecture,
        Some(mode),
        if burst { "burst_10" } else { "isolated" },
        fixture.initial_backend_cycles,
    );
    result.subscriber_kind = "global_real_receiver".into();
    result.subscriber_count = 1;
    result.backend_cycles_before = cycles_before;
    result.backend_cycles_after = cycles_after;
    result.net_cycles_consumed = cycles_before.saturating_sub(cycles_after);
    result.injected_blocks = blocks_each * samples;
    result.processed_blocks = cursor_after - cursor_before;
    result.completed_polls = samples;
    result.completed_ledger_calls = fixture.ledger_calls()? - calls_before;
    result.ledger_pages = result.completed_ledger_calls;
    result.cursor_before = cursor_before;
    result.cursor_after = cursor_after;
    result.poke_attempts = poke_counts_after.0 - poke_counts_before.0;
    result.accepted_oneway_calls = poke_counts_after.1 - poke_counts_before.1;
    result.observed_subscriber_callbacks = Some(samples);
    result.simulated_time_elapsed_seconds = now_seconds(&fixture.pic) - time_before;
    result.latency = summarize_latency(latencies);
    result.scheduler_driven = true;
    result.notes.push("Latency uses PocketIC simulated timestamps at injection and receiver observation; host wall-clock time is excluded.".into());
    if mode == Mode::Continuous {
        result.notes.push("Zero simulated seconds means no deliberate timer delay across PocketIC rounds, not one poll or notification per second on mainnet.".into());
    }
    Ok(result)
}

fn measure_thresholds(config: &RunConfig) -> Result<Vec<ThresholdObservation>> {
    let fixture = Fixture::new(Architecture::CanonicalIcp, 200 * TRILLION)?;
    let thresholds = crate::types::Thresholds::current();
    let mut out = Vec::new();
    for mode in Mode::ACTIVE {
        for (kind, boundary) in [
            ("entry", thresholds.entry(mode)),
            ("exit", thresholds.exit(mode)),
        ] {
            for (offset_name, balance) in [
                ("below", boundary.saturating_sub(1)),
                ("equal", boundary),
                ("above", boundary.saturating_add(1)),
            ] {
                fixture.set_scheduler_balance(Some(TRILLION - 1))?;
                let reset = fixture.refresh_mode()?;
                if reset != Mode::ReserveProtection {
                    bail!("threshold fixture could not reset to Reserve Protection")
                }
                let current = if kind == "exit" {
                    fixture.set_scheduler_balance(Some(thresholds.entry(mode)))?;
                    let entered = fixture.refresh_mode()?;
                    if entered != mode {
                        bail!("could not enter {mode:?} for exit-boundary test; got {entered:?}")
                    }
                    mode
                } else {
                    Mode::ReserveProtection
                };
                fixture.set_scheduler_balance(Some(balance))?;
                let observed = fixture.refresh_mode()?;
                let expected: Mode = event_horizon::next_mode(current.into(), balance).into();
                out.push(ThresholdObservation {
                    scenario_id: format!("threshold_{}_{}_{}", mode_id(mode), kind, offset_name),
                    current_mode: current,
                    balance_cycles: balance,
                    expected_mode: expected,
                    observed_mode: observed,
                    agrees: observed == expected,
                });
            }
        }
    }

    let sequence = [
        100 * TRILLION,
        80 * TRILLION,
        80 * TRILLION + 1,
        80 * TRILLION - 1,
        80 * TRILLION + 2,
        60 * TRILLION,
        60 * TRILLION - 1,
        25 * TRILLION,
        15 * TRILLION,
        15 * TRILLION - 1,
        10 * TRILLION,
        6 * TRILLION,
        6 * TRILLION - 1,
        5 * TRILLION,
        3 * TRILLION,
        3 * TRILLION - 1,
        2 * TRILLION,
        TRILLION,
        TRILLION - 1,
        2 * TRILLION,
    ];
    fixture.set_scheduler_balance(Some(TRILLION - 1))?;
    let mut current = fixture.refresh_mode()?;
    for (index, balance) in sequence.into_iter().enumerate() {
        fixture.set_scheduler_balance(Some(balance))?;
        let observed = fixture.refresh_mode()?;
        let expected: Mode = event_horizon::next_mode(current.into(), balance).into();
        out.push(ThresholdObservation {
            scenario_id: format!("threshold_transition_sequence_{index:02}"),
            current_mode: current,
            balance_cycles: balance,
            expected_mode: expected,
            observed_mode: observed,
            agrees: observed == expected,
        });
        current = observed;
    }
    if out.len() as u64 > config.limits.maximum_processed_blocks {
        bail!("threshold observation count exceeds configured general work cap")
    }
    Ok(out)
}

fn measure_large_balances(config: &RunConfig) -> Result<Vec<Measurement>> {
    ensure_poll_budget(9, config)?;
    let fixture = Fixture::new(Architecture::CanonicalIcp, TRILLION)?;
    let mut out = Vec::new();
    for trillions in [1_u128, 2, 5, 10, 25, 60, 100, 200, 1_000] {
        let target = trillions
            .checked_mul(TRILLION)
            .ok_or_else(|| anyhow!("large balance overflow"))?;
        let current = fixture.pic.cycle_balance(fixture.backend);
        if current > target {
            bail!("fixture balance {current} already exceeds target {target}")
        }
        fixture.pic.add_cycles(fixture.backend, target - current);
        let exact = fixture.pic.cycle_balance(fixture.backend);
        if exact != target {
            bail!("PocketIC balance mismatch: expected {target}, got {exact}")
        }
        let calls_before = fixture.ledger_calls()?;
        let cursor_before = fixture.state()?.observed_next_block;
        fixture.poll()?;
        let after = fixture.pic.cycle_balance(fixture.backend);
        let consumed = target.saturating_sub(after);
        if consumed == 0 {
            bail!("large-balance poll at {trillions} T recorded no expenditure")
        }
        let mut item = base_measurement(
            format!("large_balance_{trillions}T"),
            "F_large_synthetic_balances",
            fixture.architecture,
            None,
            "idle",
            target,
        );
        item.backend_cycles_before = target;
        item.backend_cycles_after = after;
        item.net_cycles_consumed = consumed;
        item.per_poll_cycles = vec![consumed];
        fill_cycle_summary(&mut item);
        item.completed_polls = 1;
        item.completed_ledger_calls = fixture.ledger_calls()? - calls_before;
        item.ledger_pages = item.completed_ledger_calls;
        item.cursor_before = cursor_before;
        item.cursor_after = fixture.state()?.observed_next_block;
        item.processed_blocks = item.cursor_after - cursor_before;
        item.notes.push(
            "Balance was an actual PocketIC canister balance; no liquid-cycle override was used."
                .into(),
        );
        out.push(item);
    }
    Ok(out)
}

fn base_measurement(
    scenario_id: String,
    experiment: &str,
    architecture: Architecture,
    polling_mode: Option<Mode>,
    traffic_profile: &str,
    initial_backend_cycles: u128,
) -> Measurement {
    Measurement {
        scenario_id,
        experiment: experiment.into(),
        architecture: architecture.id().into(),
        polling_mode,
        traffic_profile: traffic_profile.into(),
        subscriber_kind: "none".into(),
        subscriber_count: 0,
        initial_backend_cycles,
        backend_cycles_before: 0,
        backend_cycles_after: 0,
        net_cycles_consumed: 0,
        per_poll_cycles: Vec::new(),
        average_cycles_per_poll: None,
        median_cycles_per_poll: None,
        minimum_cycles_per_poll: None,
        maximum_cycles_per_poll: None,
        initial_ledger_blocks: 0,
        injected_blocks: 0,
        processed_blocks: 0,
        completed_polls: 0,
        completed_ledger_calls: 0,
        ledger_pages: 0,
        cursor_before: 0,
        cursor_after: 0,
        maximum_ledger_lag: 0,
        average_ledger_lag: 0,
        catch_up_time_seconds: None,
        poke_attempts: 0,
        accepted_oneway_calls: 0,
        observed_subscriber_callbacks: None,
        callback_payload_bytes: None,
        simulated_time_elapsed_seconds: 0,
        latency: LatencySummary::default(),
        maintenance_included: false,
        scheduler_driven: false,
        actual_cycles_balance_used: true,
        liquid_cycles_override_used: false,
        cap_reached: None,
        complete: true,
        failure: None,
        notes: Vec::new(),
    }
}

fn fill_cycle_summary(result: &mut Measurement) {
    if result.per_poll_cycles.is_empty() {
        return;
    }
    let mut sorted = result.per_poll_cycles.clone();
    sorted.sort_unstable();
    let total = sorted.iter().copied().sum::<u128>();
    result.average_cycles_per_poll = Some(total / sorted.len() as u128);
    result.median_cycles_per_poll = Some(sorted[(sorted.len() - 1) / 2]);
    result.minimum_cycles_per_poll = sorted.first().copied();
    result.maximum_cycles_per_poll = sorted.last().copied();
}

fn summarize_latency(mut samples: Vec<u64>) -> LatencySummary {
    samples.sort_unstable();
    if samples.is_empty() {
        return LatencySummary::default();
    }
    let p95_index = ((samples.len() * 95).div_ceil(100)).saturating_sub(1);
    LatencySummary {
        minimum_seconds: samples.first().copied(),
        median_seconds: Some(samples[(samples.len() - 1) / 2]),
        percentile_95_seconds: samples.get(p95_index).copied(),
        maximum_seconds: samples.last().copied(),
        samples_seconds: samples,
    }
}

fn balance_for_mode(mode: Mode) -> u128 {
    match mode {
        Mode::ReserveProtection => 1_500_000_000_000,
        Mode::Economy => 2_500_000_000_000,
        Mode::Standard => 5_500_000_000_000,
        Mode::Fast => 10_500_000_000_000,
        Mode::VeryFast => 26 * TRILLION,
        Mode::Continuous => 110 * TRILLION,
    }
}

fn mode_id(mode: Mode) -> &'static str {
    match mode {
        Mode::ReserveProtection => "reserve_protection",
        Mode::Economy => "economy",
        Mode::Standard => "standard",
        Mode::Fast => "fast",
        Mode::VeryFast => "very_fast",
        Mode::Continuous => "continuous",
    }
}

fn account(owner: Principal, subaccount: u64) -> Account {
    let mut bytes = vec![0_u8; 32];
    bytes[24..].copy_from_slice(&subaccount.to_be_bytes());
    Account {
        owner,
        subaccount: Some(bytes),
    }
}

fn now_seconds(pic: &PocketIc) -> u64 {
    pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000
}

fn update<A: CandidType, R: for<'de> Deserialize<'de> + CandidType>(
    pic: &PocketIc,
    id: Principal,
    method: &str,
    arg: A,
) -> Result<R> {
    let bytes = pic
        .update_call(id, Principal::anonymous(), method, encode_one(arg)?)
        .map_err(|error| anyhow!("{method}: {error:?}"))?;
    decode_one(&bytes).with_context(|| format!("decode {method}"))
}

fn query<A: CandidType, R: for<'de> Deserialize<'de> + CandidType>(
    pic: &PocketIc,
    id: Principal,
    method: &str,
    arg: A,
) -> Result<R> {
    let bytes = pic
        .query_call(id, Principal::anonymous(), method, encode_one(arg)?)
        .map_err(|error| anyhow!("{method}: {error:?}"))?;
    decode_one(&bytes).with_context(|| format!("decode {method}"))
}

fn workspace_root() -> Result<PathBuf> {
    for dir in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        if dir.join("Cargo.toml").exists() && dir.join("SPEC.md").exists() {
            return Ok(dir.to_path_buf());
        }
    }
    bail!("workspace root not found")
}

fn wasm(cache: &OnceLock<Vec<u8>>, package: &str, feature: Option<&str>) -> Result<Vec<u8>> {
    if let Some(bytes) = cache.get() {
        return Ok(bytes.clone());
    }
    let root = workspace_root()?;
    let mut command = Command::new("cargo");
    command.args([
        "build",
        "--locked",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
        "-p",
        package,
    ]);
    if let Some(feature) = feature {
        command.args(["--features", feature]);
    }
    let status = command
        .current_dir(&root)
        .status()
        .with_context(|| format!("build {package}"))?;
    if !status.success() {
        bail!("Wasm build failed for {package}")
    }
    let path = root.join(format!(
        "target/wasm32-unknown-unknown/release/{}.wasm",
        package.replace('-', "_")
    ));
    let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let _ = cache.set(bytes.clone());
    Ok(bytes)
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root()?)
        .output()
        .context("run git")?;
    if !output.status.success() {
        bail!("git {} failed", args.join(" "))
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn git_status_clean() -> Result<bool> {
    Ok(git(&["status", "--porcelain"])?.is_empty())
}

fn ensure_wall_budget(started: Instant, seconds: u64) -> Result<()> {
    if started.elapsed() > Duration::from_secs(seconds) {
        bail!("calibration exceeded configured wall-clock budget of {seconds} seconds")
    }
    Ok(())
}

fn ensure_poll_budget(polls: u64, config: &RunConfig) -> Result<()> {
    if polls > config.limits.maximum_completed_polls {
        bail!(
            "scenario requests {polls} completed polls, over configured cap {}",
            config.limits.maximum_completed_polls
        )
    }
    Ok(())
}

fn write_measurements(set: &MeasurementSet, output: &Path) -> Result<()> {
    let json = serde_json::to_vec_pretty(set)?;
    fs::write(output.join("measurements.json"), json)?;
    let mut csv = String::from("scenario_id,experiment,architecture,mode,traffic_profile,subscriber_kind,subscriber_count,completed_polls,ledger_calls,processed_blocks,poke_attempts,accepted_oneway_calls,cycles_before,cycles_after,cycles_consumed,average_cycles_per_poll,median_cycles_per_poll,simulated_seconds,complete\n");
    for item in &set.scenarios {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_field(&item.scenario_id),
            csv_field(&item.experiment),
            csv_field(&item.architecture),
            item.polling_mode.map(mode_id).unwrap_or(""),
            csv_field(&item.traffic_profile),
            csv_field(&item.subscriber_kind),
            item.subscriber_count,
            item.completed_polls,
            item.completed_ledger_calls,
            item.processed_blocks,
            item.poke_attempts,
            item.accepted_oneway_calls,
            item.backend_cycles_before,
            item.backend_cycles_after,
            item.net_cycles_consumed,
            item.average_cycles_per_poll.unwrap_or_default(),
            item.median_cycles_per_poll.unwrap_or_default(),
            item.simulated_time_elapsed_seconds,
            item.complete,
        ));
    }
    fs::write(output.join("measurements.csv"), csv)?;
    Ok(())
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

pub fn read_measurements(path: &Path) -> Result<MeasurementSet> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let set: MeasurementSet =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    validate_measurements(&set)?;
    Ok(set)
}

pub fn validate_measurements(set: &MeasurementSet) -> Result<()> {
    if set.schema_version != 1 {
        bail!("unsupported measurements schema {}", set.schema_version)
    }
    if set.scenarios.is_empty() {
        bail!("measurements contain no scenarios")
    }
    for item in &set.scenarios {
        if !item.complete || item.failure.is_some() || item.cap_reached.is_some() {
            bail!(
                "scenario {} is incomplete: failure={:?} cap={:?}",
                item.scenario_id,
                item.failure,
                item.cap_reached
            )
        }
        if item.backend_cycles_after > item.backend_cycles_before {
            bail!(
                "scenario {} has increasing balance inside a burn measurement",
                item.scenario_id
            )
        }
        if item.net_cycles_consumed != item.backend_cycles_before - item.backend_cycles_after {
            bail!(
                "scenario {} has inconsistent cycle arithmetic",
                item.scenario_id
            )
        }
    }
    if set.threshold_observations.iter().any(|item| !item.agrees) {
        bail!("threshold observations contain a production/model mismatch")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_summary_uses_nearest_rank_percentile() {
        let summary = summarize_latency(vec![5, 1, 4, 2, 3]);
        assert_eq!(summary.minimum_seconds, Some(1));
        assert_eq!(summary.median_seconds, Some(3));
        assert_eq!(summary.percentile_95_seconds, Some(5));
        assert_eq!(summary.maximum_seconds, Some(5));
    }

    #[test]
    fn incomplete_measurements_are_rejected() {
        let mut item = base_measurement(
            "incomplete".into(),
            "test",
            Architecture::CanonicalIcp,
            None,
            "idle",
            10,
        );
        item.complete = false;
        let mut set = MeasurementSet {
            schema_version: 1,
            provenance: Provenance {
                git_commit: "test".into(),
                working_tree_dirty: true,
                debug_wasm_sha256: "00".into(),
                pocketic_version: "15.0.0".into(),
                subnet_topology: "test".into(),
                cycles_cost_schedule: "test".into(),
                seed: 1,
                profile: "test".into(),
                generated_at_unix_seconds: 0,
                limits: ExecutionLimits {
                    maximum_ticks_per_wait: 1,
                    maximum_completed_polls: 1,
                    maximum_processed_blocks: 1,
                    maximum_wall_seconds: 1,
                },
            },
            scenarios: vec![item],
            threshold_observations: vec![],
            limitations: vec![],
        };
        assert!(validate_measurements(&set).is_err());
        set.scenarios[0].complete = true;
        set.scenarios[0].backend_cycles_before = 10;
        set.scenarios[0].backend_cycles_after = 11;
        assert!(validate_measurements(&set).is_err());
        set.scenarios[0].backend_cycles_after = 10;
        assert!(crate::model::calibrate(&set).is_err());
    }
}
