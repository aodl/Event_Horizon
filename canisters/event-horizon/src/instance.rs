use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::clients::icrc3;
use crate::{config, state};

#[derive(Debug)]
pub enum ProfileError {
    ReserveProtection,
    ObservedLedger(String),
    SnsRoot(String),
    InvalidSns(String),
}
impl From<String> for ProfileError {
    fn from(value: String) -> Self {
        if config::is_reserve_protection(&value) {
            Self::ReserveProtection
        } else {
            Self::ObservedLedger(value)
        }
    }
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct InitArgs {
    pub observed_ledger: Principal,
    pub sns_root: Option<Principal>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObservedLedgerProfile {
    pub symbol: String,
    pub decimals: u8,
    pub supports_icrc2_transfer_from: bool,
    pub neuron_governance: Option<Principal>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct InstanceConfig {
    pub observed_ledger: Principal,
    pub observed_profile: Option<ObservedLedgerProfile>,
    pub sns_root: Option<Principal>,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct InstanceInfo {
    pub observed_ledger: Principal,
    pub observed_profile: Option<ObservedLedgerProfile>,
    pub sns_root: Option<Principal>,
    pub icp_ledger: Principal,
    pub cmc: Principal,
    pub jupiter_faucet: Principal,
    pub jupiter_historian: Principal,
    pub surplus_canister: Option<Principal>,
}

pub fn validate_init(
    observed_ledger: Principal,
    sns_root: Option<Principal>,
    icp_ledger: Principal,
) {
    assert!(
        observed_ledger != Principal::anonymous(),
        "observed_ledger must not be anonymous"
    );
    assert!(
        observed_ledger != Principal::management_canister(),
        "observed_ledger must not be the management canister"
    );
    assert!(
        observed_ledger != icp_ledger || sns_root.is_none(),
        "canonical ICP must not configure sns_root"
    );
    if let Some(root) = sns_root {
        assert!(
            root != Principal::anonymous() && root != Principal::management_canister(),
            "sns_root must be a canister principal"
        );
    }
}

#[cfg(not(feature = "debug_api"))]
pub fn log_config() {
    let runtime = config::runtime();
    let reader = if runtime.observed_ledger == runtime.icp_ledger {
        "icp_legacy"
    } else {
        "icrc3"
    };
    let sns_root = state::read_instance_config().sns_root;
    ic_cdk::println!("CONFIG instance={} observed_ledger={} sns_root={} reader={} icp_ledger={} faucet={} historian={} cmc={} surplus={}", ic_cdk::api::canister_self(), runtime.observed_ledger, sns_root.map_or_else(|| "none".into(), |p| p.to_text()), reader, runtime.icp_ledger, runtime.faucet_canister, runtime.historian_canister, runtime.cmc_canister, runtime.surplus_canister.map_or_else(|| "none".to_string(), |p| p.to_text()));
}

pub fn get_instance() -> InstanceInfo {
    let instance = state::read_instance_config();
    let runtime = config::runtime();
    InstanceInfo {
        observed_ledger: instance.observed_ledger,
        observed_profile: instance.observed_profile,
        sns_root: instance.sns_root,
        icp_ledger: runtime.icp_ledger,
        cmc: runtime.cmc_canister,
        jupiter_faucet: runtime.faucet_canister,
        jupiter_historian: runtime.historian_canister,
        surplus_canister: runtime.surplus_canister,
    }
}

pub async fn ensure_observed_profile() -> Result<ObservedLedgerProfile, ProfileError> {
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
            return Err(ProfileError::ObservedLedger(format!(
                "observed ledger does not advertise {required}"
            )));
        }
    }
    let symbol = icrc3::symbol(ledger).await?;
    if symbol.is_empty() || symbol.len() > 32 {
        return Err(ProfileError::ObservedLedger(
            "observed ledger symbol must be 1..=32 UTF-8 bytes".into(),
        ));
    }
    let decimals = icrc3::decimals(ledger).await?;
    let block_types = if is_icp {
        Vec::new()
    } else {
        icrc3::supported_block_types(ledger).await?
    };
    if !is_icp && !block_types.iter().any(|kind| kind.block_type == "1xfer") {
        return Err(ProfileError::ObservedLedger(
            "observed ledger does not advertise 1xfer".into(),
        ));
    }
    let neuron_governance = if is_icp {
        Some(Principal::from_text(config::NNS_GOVERNANCE_CANISTER).expect("valid NNS Governance"))
    } else if let Some(root) = instance.sns_root {
        #[derive(CandidType, Deserialize)]
        struct SnsCanisters {
            root: Option<Principal>,
            ledger: Option<Principal>,
            governance: Option<Principal>,
        }
        #[derive(CandidType)]
        struct Empty {}
        let call = ic_cdk::call::Call::bounded_wait(root, "list_sns_canisters").with_arg(&Empty {});
        if ic_cdk::api::canister_liquid_cycle_balance()
            < config::RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
        {
            return Err(ProfileError::ReserveProtection);
        }
        let response: SnsCanisters = call
            .await
            .map_err(|e| ProfileError::SnsRoot(format!("transport: {e:?}")))?
            .candid()
            .map_err(|e| ProfileError::SnsRoot(format!("decode: {e:?}")))?;
        let governance = response
            .governance
            .ok_or_else(|| ProfileError::InvalidSns("governance absent".into()))?;
        if response.root != Some(root)
            || response.ledger != Some(ledger)
            || governance == Principal::anonymous()
            || governance == Principal::management_canister()
        {
            return Err(ProfileError::InvalidSns(
                "Root/Ledger/Governance mismatch".into(),
            ));
        }
        let minting = icrc3::minting_account(ledger)
            .await?
            .ok_or_else(|| ProfileError::InvalidSns("minting account absent".into()))?;
        if minting.owner != governance || *minting.effective_subaccount() != [0; 32] {
            return Err(ProfileError::InvalidSns(
                "minting account is not Governance default account".into(),
            ));
        }
        Some(governance)
    } else {
        None
    };
    let profile = ObservedLedgerProfile {
        symbol,
        decimals,
        supports_icrc2_transfer_from: if is_icp {
            standards.iter().any(|s| s.name == "ICRC-2")
        } else {
            block_types.iter().any(|kind| kind.block_type == "2xfer")
        },
        neuron_governance,
    };
    crate::logging::sns_profile_recovered();
    state::write_observed_profile(profile.clone());
    ic_cdk::println!(
        "CONFIG observed_ledger={} symbol={} decimals={} icrc2_transfer_from={} neuron_governance={}",
        ledger,
        profile.symbol,
        profile.decimals,
        profile.supports_icrc2_transfer_from,
        profile.neuron_governance.map_or_else(|| "none".into(), |p| p.to_text())
    );
    Ok(profile)
}
