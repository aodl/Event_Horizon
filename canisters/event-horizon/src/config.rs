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

pub const MIN_TRIGGER_E8S: u64 = 1_000_000; // 0.01 ICP
pub const LEDGER_PAGE_SIZE: u64 = 256;
pub const FUNDING_MAINTENANCE_SECONDS: u64 = 60 * 60;
pub const RESERVE_RECHECK_SECONDS: u64 = 60 * 60;
pub const TOP_UP_CANISTER_MEMO: u64 = 1_347_768_404;

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

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub ledger_canister: Principal,
    pub cmc_canister: Principal,
    pub historian_canister: Principal,
    pub faucet_canister: Principal,
}

impl RuntimeConfig {
    pub fn production() -> Self {
        Self {
            ledger_canister: Principal::from_text(ICP_LEDGER_CANISTER)
                .expect("valid ICP Ledger principal"),
            cmc_canister: Principal::from_text(CMC_CANISTER).expect("valid CMC principal"),
            historian_canister: Principal::from_text(JUPITER_HISTORIAN_CANISTER)
                .expect("valid Historian principal"),
            faucet_canister: Principal::from_text(JUPITER_FAUCET_CANISTER)
                .expect("valid Faucet principal"),
        }
    }
}

#[cfg(not(feature = "debug_api"))]
pub fn runtime() -> RuntimeConfig {
    RuntimeConfig::production()
}

#[cfg(feature = "debug_api")]
pub fn runtime() -> RuntimeConfig {
    crate::state::read_debug_config()
}
