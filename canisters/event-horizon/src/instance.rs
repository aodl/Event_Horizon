use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::{config, state};

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct InitArgs {
    pub observed_ledger: Principal,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObservedLedgerProfile {
    pub symbol: String,
    pub decimals: u8,
    pub supports_icrc2_transfer_from: bool,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct InstanceConfig {
    pub observed_ledger: Principal,
    pub observed_profile: Option<ObservedLedgerProfile>,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct InstanceInfo {
    pub observed_ledger: Principal,
    pub observed_profile: Option<ObservedLedgerProfile>,
    pub icp_ledger: Principal,
    pub cmc: Principal,
    pub jupiter_faucet: Principal,
    pub jupiter_historian: Principal,
    pub surplus_canister: Option<Principal>,
}

pub fn validate_observed_ledger(principal: Principal) {
    assert!(
        principal != Principal::anonymous(),
        "observed_ledger must not be anonymous"
    );
    assert!(
        principal != Principal::management_canister(),
        "observed_ledger must not be the management canister"
    );
}

pub fn get_instance() -> InstanceInfo {
    let instance = state::read_instance_config();
    let runtime = config::runtime();
    InstanceInfo {
        observed_ledger: instance.observed_ledger,
        observed_profile: instance.observed_profile,
        icp_ledger: runtime.icp_ledger,
        cmc: runtime.cmc_canister,
        jupiter_faucet: runtime.faucet_canister,
        jupiter_historian: runtime.historian_canister,
        surplus_canister: runtime.surplus_canister,
    }
}
