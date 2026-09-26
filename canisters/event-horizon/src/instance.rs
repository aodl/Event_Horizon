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

#[cfg(not(feature = "debug_api"))]
pub fn log_config() {
    let runtime = config::runtime();
    let reader = if runtime.observed_ledger == runtime.icp_ledger {
        "icp_legacy"
    } else {
        "icrc3"
    };
    ic_cdk::println!("CONFIG instance={} observed_ledger={} reader={} icp_ledger={} faucet={} historian={} cmc={} surplus={}", ic_cdk::api::canister_self(), runtime.observed_ledger, reader, runtime.icp_ledger, runtime.faucet_canister, runtime.historian_canister, runtime.cmc_canister, runtime.surplus_canister.map_or_else(|| "none".to_string(), |p| p.to_text()));
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
    let runtime = config::runtime();
    let is_icp = ledger == runtime.icp_ledger;
    for required in if is_icp {
        &["ICRC-1"][..]
    } else {
        &["ICRC-1", "ICRC-3"][..]
    } {
        if !standards.iter().any(|standard| standard.name == *required) {
            return Err(format!("observed ledger does not advertise {required}"));
        }
    }
    let symbol = icrc3::symbol(ledger).await?;
    if symbol.is_empty() || symbol.len() > 32 {
        return Err("observed ledger symbol must be 1..=32 UTF-8 bytes".into());
    }
    let decimals = icrc3::decimals(ledger).await?;
    let block_types = if is_icp {
        Vec::new()
    } else {
        icrc3::supported_block_types(ledger).await?
    };
    if !is_icp && !block_types.iter().any(|kind| kind.block_type == "1xfer") {
        return Err("observed ledger does not advertise 1xfer".into());
    }
    let profile = ObservedLedgerProfile {
        symbol,
        decimals,
        supports_icrc2_transfer_from: if is_icp {
            standards.iter().any(|s| s.name == "ICRC-2")
        } else {
            block_types.iter().any(|kind| kind.block_type == "2xfer")
        },
    };
    state::write_observed_profile(profile.clone());
    ic_cdk::println!(
        "CONFIG observed_ledger={} symbol={} decimals={} icrc2_transfer_from={}",
        ledger,
        profile.symbol,
        profile.decimals,
        profile.supports_icrc2_transfer_from
    );
    Ok(profile)
}
