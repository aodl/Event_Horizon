use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::clients::icrc3;
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

pub async fn ensure_observed_profile() -> Result<ObservedLedgerProfile, String> {
    let instance = state::read_instance_config();
    if let Some(profile) = instance.observed_profile {
        return Ok(profile);
    }
    let ledger = instance.observed_ledger;
    let standards = icrc3::supported_standards(ledger).await?;
    for required in ["ICRC-1", "ICRC-3"] {
        if !standards.iter().any(|standard| standard.name == required) {
            return Err(format!("observed ledger does not advertise {required}"));
        }
    }
    let symbol = icrc3::symbol(ledger).await?;
    if symbol.is_empty() || symbol.len() > 32 {
        return Err("observed ledger symbol must be 1..=32 UTF-8 bytes".into());
    }
    let decimals = icrc3::decimals(ledger).await?;
    let block_types = icrc3::supported_block_types(ledger).await?;
    if !block_types.iter().any(|kind| kind.block_type == "1xfer") {
        return Err("observed ledger does not advertise 1xfer".into());
    }
    let profile = ObservedLedgerProfile {
        symbol,
        decimals,
        supports_icrc2_transfer_from: block_types.iter().any(|kind| kind.block_type == "2xfer"),
    };
    state::write_observed_profile(profile.clone());
    Ok(profile)
}
