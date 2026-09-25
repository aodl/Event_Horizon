//! Fixed production policy/trust anchors.
//!
//! These values intentionally compile into the production Wasm so its module hash
//! commits to protocol configuration as well as executable behaviour.

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

pub const ICP_LEDGER_CANISTER: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const CMC_CANISTER: &str = "rkp4c-7iaaa-aaaaa-aaaca-cai";
pub const JUPITER_HISTORIAN_CANISTER: &str = "j5gs6-uiaaa-aaaar-qb5cq-cai";
pub const JUPITER_FAUCET_CANISTER: &str = "acjuz-liaaa-aaaar-qb4qq-cai";
/// Immutable production surplus destination. `None` keeps diversion disabled.
pub const SURPLUS_CANISTER: Option<&str> = None;

pub const MIN_TRIGGER_E8S: u64 = 1_000_000; // 0.01 ICP
pub const LEDGER_PAGE_SIZE: u64 = 256;
pub const FUNDING_MAINTENANCE_SECONDS: u64 = 60 * 60;
pub const RESERVE_RECHECK_SECONDS: u64 = 60 * 60;
pub const TOP_UP_CANISTER_MEMO: u64 = 1_347_768_404;
/// Big-endian ASCII `SURPLUS1`; identifies only this protocol's surplus transfers.
pub const SURPLUS_TRANSFER_MEMO: u64 = u64::from_be_bytes(*b"SURPLUS1");

pub const TRILLION: u128 = 1_000_000_000_000;
pub const RESERVE_PROTECTION_CYCLES: u128 = TRILLION;
pub const ECONOMY_ENTER: u128 = 2 * TRILLION;
pub const ECONOMY_EXIT: u128 = TRILLION;
pub const STANDARD_ENTER: u128 = 5 * TRILLION;
pub const STANDARD_EXIT: u128 = 3 * TRILLION;
pub const FAST_ENTER: u128 = 10 * TRILLION;
pub const FAST_EXIT: u128 = 6 * TRILLION;
pub const VERY_FAST_ENTER: u128 = 25 * TRILLION;
pub const VERY_FAST_EXIT: u128 = 15 * TRILLION;
pub const CONTINUOUS_ENTER: u128 = 100 * TRILLION;
pub const CONTINUOUS_EXIT: u128 = 60 * TRILLION;

pub const SURPLUS_HEALTH_THRESHOLD_CYCLES: u128 = 150 * TRILLION;
pub const SURPLUS_EMERGENCY_THRESHOLD_CYCLES: u128 = 100 * TRILLION;
pub const SURPLUS_EPOCH_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const SURPLUS_STEP_PERCENT: u8 = 5;
pub const SURPLUS_MAX_PERCENT: u8 = 95;
pub const SURPLUS_MAX_LEVEL: u8 = SURPLUS_MAX_PERCENT / SURPLUS_STEP_PERCENT;

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub observed_ledger: Principal,
    pub icp_ledger: Principal,
    pub cmc_canister: Principal,
    pub historian_canister: Principal,
    pub faucet_canister: Principal,
    pub surplus_canister: Option<Principal>,
}

impl RuntimeConfig {
    pub fn production(observed_ledger: Principal) -> Self {
        Self {
            observed_ledger,
            icp_ledger: Principal::from_text(ICP_LEDGER_CANISTER)
                .expect("valid ICP Ledger principal"),
            cmc_canister: Principal::from_text(CMC_CANISTER).expect("valid CMC principal"),
            historian_canister: Principal::from_text(JUPITER_HISTORIAN_CANISTER)
                .expect("valid Historian principal"),
            faucet_canister: Principal::from_text(JUPITER_FAUCET_CANISTER)
                .expect("valid Faucet principal"),
            surplus_canister: SURPLUS_CANISTER.map(|principal| {
                Principal::from_text(principal).expect("valid surplus canister principal")
            }),
        }
    }
}

#[cfg(not(feature = "debug_api"))]
pub fn runtime() -> RuntimeConfig {
    RuntimeConfig::production(crate::state::read_instance_config().observed_ledger)
}

#[cfg(feature = "debug_api")]
pub fn runtime() -> RuntimeConfig {
    crate::state::read_debug_config()
}
