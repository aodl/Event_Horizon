use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    ReserveProtection,
    Economy,
    Standard,
    Fast,
    VeryFast,
    Continuous,
}

impl Mode {
    pub const ALL: [Self; 6] = [
        Self::ReserveProtection,
        Self::Economy,
        Self::Standard,
        Self::Fast,
        Self::VeryFast,
        Self::Continuous,
    ];

    pub const ACTIVE: [Self; 5] = [
        Self::Economy,
        Self::Standard,
        Self::Fast,
        Self::VeryFast,
        Self::Continuous,
    ];

    pub const fn delay_seconds(self, continuous_interval_seconds: u64) -> Option<u64> {
        match self {
            Self::ReserveProtection => None,
            Self::Economy => Some(3_600),
            Self::Standard => Some(600),
            Self::Fast => Some(120),
            Self::VeryFast => Some(10),
            Self::Continuous => Some(continuous_interval_seconds),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ReserveProtection => "Reserve Protection",
            Self::Economy => "Economy",
            Self::Standard => "Standard",
            Self::Fast => "Fast",
            Self::VeryFast => "Very Fast",
            Self::Continuous => "Continuous",
        }
    }
}

impl From<event_horizon::PollingMode> for Mode {
    fn from(value: event_horizon::PollingMode) -> Self {
        match value {
            event_horizon::PollingMode::ReserveProtection => Self::ReserveProtection,
            event_horizon::PollingMode::Economy => Self::Economy,
            event_horizon::PollingMode::Standard => Self::Standard,
            event_horizon::PollingMode::Fast => Self::Fast,
            event_horizon::PollingMode::VeryFast => Self::VeryFast,
            event_horizon::PollingMode::Continuous => Self::Continuous,
        }
    }
}

impl From<Mode> for event_horizon::PollingMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::ReserveProtection => Self::ReserveProtection,
            Mode::Economy => Self::Economy,
            Mode::Standard => Self::Standard,
            Mode::Fast => Self::Fast,
            Mode::VeryFast => Self::VeryFast,
            Mode::Continuous => Self::Continuous,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Thresholds {
    pub economy_entry: u128,
    pub economy_exit: u128,
    pub standard_entry: u128,
    pub standard_exit: u128,
    pub fast_entry: u128,
    pub fast_exit: u128,
    pub very_fast_entry: u128,
    pub very_fast_exit: u128,
    pub continuous_entry: u128,
    pub continuous_exit: u128,
}

impl Thresholds {
    pub const fn current() -> Self {
        use crate::TRILLION as T;
        Self {
            economy_entry: 2 * T,
            economy_exit: T,
            standard_entry: 5 * T,
            standard_exit: 3 * T,
            fast_entry: 10 * T,
            fast_exit: 6 * T,
            very_fast_entry: 25 * T,
            very_fast_exit: 15 * T,
            continuous_entry: 100 * T,
            continuous_exit: 60 * T,
        }
    }

    pub const fn conservative() -> Self {
        use crate::TRILLION as T;
        Self {
            economy_entry: 3 * T,
            economy_exit: T,
            standard_entry: 8 * T,
            standard_exit: 4 * T,
            fast_entry: 15 * T,
            fast_exit: 9 * T,
            very_fast_entry: 40 * T,
            very_fast_exit: 24 * T,
            continuous_entry: 160 * T,
            continuous_exit: 90 * T,
        }
    }

    pub const fn candidate_125t() -> Self {
        let mut value = Self::current();
        value.continuous_entry = 125 * crate::TRILLION;
        value
    }

    pub const fn performance() -> Self {
        use crate::TRILLION as T;
        Self {
            economy_entry: 2 * T,
            economy_exit: T,
            standard_entry: 4 * T,
            standard_exit: 2_500_000_000_000,
            fast_entry: 8 * T,
            fast_exit: 5 * T,
            very_fast_entry: 20 * T,
            very_fast_exit: 12 * T,
            continuous_entry: 75 * T,
            continuous_exit: 45 * T,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let entries = [
            self.economy_entry,
            self.standard_entry,
            self.fast_entry,
            self.very_fast_entry,
            self.continuous_entry,
        ];
        let exits = [
            self.economy_exit,
            self.standard_exit,
            self.fast_exit,
            self.very_fast_exit,
            self.continuous_exit,
        ];
        if self.economy_exit != crate::TRILLION {
            return Err("Reserve Protection floor must remain 1 T".into());
        }
        if !entries.windows(2).all(|pair| pair[0] < pair[1])
            || !exits.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err("entry and exit thresholds must be strictly ordered".into());
        }
        if !entries.iter().zip(exits).all(|(entry, exit)| *entry > exit) {
            return Err("every entry threshold must exceed its exit threshold".into());
        }
        Ok(())
    }

    pub const fn entry(&self, mode: Mode) -> u128 {
        match mode {
            Mode::ReserveProtection => 0,
            Mode::Economy => self.economy_entry,
            Mode::Standard => self.standard_entry,
            Mode::Fast => self.fast_entry,
            Mode::VeryFast => self.very_fast_entry,
            Mode::Continuous => self.continuous_entry,
        }
    }

    pub const fn exit(&self, mode: Mode) -> u128 {
        match mode {
            Mode::ReserveProtection => 0,
            Mode::Economy => self.economy_exit,
            Mode::Standard => self.standard_exit,
            Mode::Fast => self.fast_exit,
            Mode::VeryFast => self.very_fast_exit,
            Mode::Continuous => self.continuous_exit,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Policy {
    pub id: String,
    pub thresholds: Thresholds,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutionLimits {
    pub maximum_ticks_per_wait: u64,
    pub maximum_completed_polls: u64,
    pub maximum_processed_blocks: u64,
    pub maximum_wall_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Provenance {
    pub git_commit: String,
    pub working_tree_dirty: bool,
    pub debug_wasm_sha256: String,
    pub pocketic_version: String,
    pub subnet_topology: String,
    pub cycles_cost_schedule: String,
    pub seed: u64,
    pub profile: String,
    pub generated_at_unix_seconds: u64,
    pub limits: ExecutionLimits,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LatencySummary {
    pub samples_seconds: Vec<u64>,
    pub minimum_seconds: Option<u64>,
    pub median_seconds: Option<u64>,
    pub percentile_95_seconds: Option<u64>,
    pub maximum_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Measurement {
    pub scenario_id: String,
    pub experiment: String,
    pub architecture: String,
    pub polling_mode: Option<Mode>,
    pub traffic_profile: String,
    pub subscriber_kind: String,
    pub subscriber_count: u64,
    pub initial_backend_cycles: u128,
    pub backend_cycles_before: u128,
    pub backend_cycles_after: u128,
    pub net_cycles_consumed: u128,
    pub per_poll_cycles: Vec<u128>,
    pub average_cycles_per_poll: Option<u128>,
    pub median_cycles_per_poll: Option<u128>,
    pub minimum_cycles_per_poll: Option<u128>,
    pub maximum_cycles_per_poll: Option<u128>,
    pub initial_ledger_blocks: u64,
    pub injected_blocks: u64,
    pub processed_blocks: u64,
    pub completed_polls: u64,
    pub completed_ledger_calls: u64,
    pub ledger_pages: u64,
    pub cursor_before: u64,
    pub cursor_after: u64,
    pub maximum_ledger_lag: u64,
    pub average_ledger_lag: u64,
    pub catch_up_time_seconds: Option<u64>,
    pub poke_attempts: u64,
    pub accepted_oneway_calls: u64,
    pub observed_subscriber_callbacks: Option<u64>,
    pub callback_payload_bytes: Option<u64>,
    pub simulated_time_elapsed_seconds: u64,
    pub latency: LatencySummary,
    pub maintenance_included: bool,
    pub scheduler_driven: bool,
    pub actual_cycles_balance_used: bool,
    pub liquid_cycles_override_used: bool,
    pub cap_reached: Option<String>,
    pub complete: bool,
    pub failure: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThresholdObservation {
    pub scenario_id: String,
    pub current_mode: Mode,
    pub balance_cycles: u128,
    pub expected_mode: Mode,
    pub observed_mode: Mode,
    pub agrees: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MeasurementSet {
    pub schema_version: u32,
    pub provenance: Provenance,
    pub scenarios: Vec<Measurement>,
    pub threshold_observations: Vec<ThresholdObservation>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CostCalibration {
    pub architecture: String,
    pub empty_poll_cycles: u128,
    pub cycles_per_processed_block: u128,
    pub cycles_per_ledger_page: u128,
    pub cycles_per_subscriber_notification: u128,
    pub funding_maintenance_cycles: Option<u128>,
    pub pricing_maintenance_cycles: Option<u128>,
    pub largest_completed_backlog: u64,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FundingAssumptions {
    pub account_reference_endowment_icp_e8s: u128,
    pub range_reference_endowment_icp_e8s: u128,
    pub global_reference_endowment_icp_e8s: u128,
    pub gross_maturity_apy_basis_points: u64,
    pub effective_payout_apy_basis_points: u64,
    pub xdr_per_icp_permyriad: u64,
    pub initial_recognition_delay_seconds: u64,
    pub conversion_delay_seconds: u64,
    pub conversion_fee_e8s: u64,
    pub delivery_shortfall_basis_points: u64,
    pub surplus_diversion_basis_points: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProductiveEndowmentBacking {
    pub protocol_endowment_icp_e8s: u128,
    pub account_endowments: u64,
    pub range_endowments: u64,
    pub global_endowments: u64,
    pub admission_price_basis_points_of_reference: u64,
    pub coupled_to_subscriber_workload: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Workload {
    pub id: String,
    pub architecture: String,
    pub blocks_per_second_milli: u64,
    pub specific_matching_blocks_basis_points: u64,
    pub specific_subscribers_per_matching_block_milli: u64,
    pub global_subscribers: u64,
    pub specific_subscribers: u64,
    pub global_specific_overlap_subscribers: u64,
    pub initial_backlog_blocks: u64,
    /// Explicit modelling capacity; the largest completed PocketIC backlog is evidence, not a protocol cap.
    pub assumed_blocks_per_poll_capacity: u64,
    pub productive_endowment_backing: ProductiveEndowmentBacking,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModelConfig {
    pub funding_assumptions: FundingAssumptions,
    pub policies: Vec<Policy>,
    pub workloads: Vec<Workload>,
    pub continuous_effective_poll_interval_seconds: u64,
    pub continuous_effective_poll_intervals_seconds: Vec<u64>,
    pub equal_starting_balances_cycles: Vec<u128>,
    pub continuous_runway_targets_days: Vec<u64>,
    pub lower_tier_runway_targets_days: [u64; 4],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimulationResult {
    pub scenario_id: String,
    pub simulation_kind: String,
    pub policy_id: String,
    pub funding_profile: String,
    pub workload_id: String,
    pub horizon_days: u64,
    pub initial_cycles: u128,
    pub initial_mode: Mode,
    pub continuous_effective_poll_interval_seconds: u64,
    pub total_funding_cycles: u128,
    pub total_projected_expenditure_cycles: u128,
    pub total_ordinary_polling_cycles: u128,
    pub total_maintenance_cycles: u128,
    pub funding_maintenance_runs: u64,
    pub pricing_maintenance_runs: u64,
    pub global_notifications: u128,
    pub specific_notifications: u128,
    pub coalesced_notification_dispatches: u128,
    pub ending_cycles: u128,
    pub minimum_cycles: u128,
    pub seconds_in_modes: [u64; 6],
    pub tier_transitions: u64,
    pub maximum_transitions_in_rolling_seven_days: u64,
    pub first_downgrade_seconds: Option<u64>,
    pub reserve_protection_seconds: Option<u64>,
    pub ending_backlog_blocks: u128,
    pub maximum_backlog_blocks: u128,
    pub maximum_active_pre_poll_pending_blocks: u128,
    pub maximum_active_post_poll_outstanding_blocks: u128,
    pub outstanding_blocks_at_first_downgrade: Option<u128>,
    pub outstanding_blocks_at_first_reserve: Option<u128>,
    pub additional_reserve_backlog_blocks: u128,
    pub projected_latency_seconds: u64,
    pub funding_shortfall_cycles: u128,
    pub funding_surplus_cycles: u128,
    pub liquidity_maintained: bool,
    pub window_funding_covers_expenditure: bool,
    pub exhausted: bool,
    pub continuous_runway_seconds: Option<u64>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixedTierEconomics {
    pub workload_id: String,
    pub mode: Mode,
    pub effective_poll_interval_seconds: u64,
    pub payout_cadence_days: u64,
    pub annual_generated_blocks: u128,
    pub annual_processed_blocks: u128,
    pub mature_annual_income_cycles: u128,
    pub annualized_operating_expenditure_cycles: u128,
    pub steady_state_annual_margin_cycles: i128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimulationSet {
    pub schema_version: u32,
    pub source_measurements: String,
    pub seed: u64,
    pub calibration: CostCalibration,
    pub architecture_calibrations: Vec<CostCalibration>,
    pub funding_assumptions: FundingAssumptions,
    pub policies: Vec<Policy>,
    pub workloads: Vec<Workload>,
    pub continuous_effective_poll_interval_seconds: u64,
    pub continuous_effective_poll_intervals_seconds: Vec<u64>,
    pub equal_starting_balances_cycles: Vec<u128>,
    pub continuous_runway_targets_days: Vec<u64>,
    pub lower_tier_runway_targets_days: [u64; 4],
    pub fixed_tier_economics: Vec<FixedTierEconomics>,
    pub results: Vec<SimulationResult>,
    pub limitations: Vec<String>,
}
