use std::{borrow::Cow, cell::RefCell};

use candid::{decode_one, encode_one, CandidType};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    DefaultMemoryImpl, StableBTreeMap, StableCell, Storable,
};
use serde::{Deserialize, Serialize};

use crate::account::{account_identifier_bytes, numbered_subaccount};
#[cfg(feature = "debug_api")]
use crate::config::RuntimeConfig;
use crate::instance::InstanceConfig;
use crate::{
    cadence::PollingMode,
    pricing::{Observation, PricingState},
    subscription::Subscription,
    surplus::SurplusPolicyState,
};
use icrc_ledger_types::icrc1::account::Account;

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Stable-memory IDs are part of the long-lived storage contract. Never reuse them.
const META_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBSCRIPTIONS_MEMORY_ID: MemoryId = MemoryId::new(1);
const INSTANCE_CONFIG_MEMORY_ID: MemoryId = MemoryId::new(2);
#[cfg(feature = "debug_api")]
const DEBUG_CONFIG_MEMORY_ID: MemoryId = MemoryId::new(3);
const GLOBAL_SUBSCRIBERS_MEMORY_ID: MemoryId = MemoryId::new(4);
const PRICE_OBSERVATIONS_MEMORY_ID: MemoryId = MemoryId::new(5);
const PRICING_STATE_MEMORY_ID: MemoryId = MemoryId::new(6);
const FUNDING_MEMORY_ID: MemoryId = MemoryId::new(7);
const SURPLUS_POLICY_MEMORY_ID: MemoryId = MemoryId::new(8);

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub admission_bootstrapped: bool,
    pub admission_next_block: u64,
    pub observed_bootstrapped: bool,
    pub observed_next_block: u64,
    pub polling_mode: PollingMode,
    pub last_health_log_day: Option<u64>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            admission_bootstrapped: false,
            admission_next_block: 0,
            observed_bootstrapped: false,
            observed_next_block: 0,
            polling_mode: PollingMode::ReserveProtection,
            last_health_log_day: None,
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedSurplus {
    pub destination: [u8; 32],
    pub memo: u64,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum FundingState {
    #[default]
    Idle,
    CmcTransferPending {
        amount_e8s: u64,
        fee_e8s: u64,
        created_at_time_nanos: u64,
        planned_surplus: Option<PlannedSurplus>,
    },
    CmcNotifyPending {
        block_index: u64,
        planned_surplus: Option<PlannedSurplus>,
    },
    SurplusTransferPending {
        destination: [u8; 32],
        memo: u64,
        amount_e8s: u64,
        fee_e8s: u64,
        created_at_time_nanos: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AccountKey(Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PrincipalKey(Vec<u8>);

impl From<candid::Principal> for PrincipalKey {
    fn from(value: candid::Principal) -> Self {
        Self(value.as_slice().to_vec())
    }
}

impl Storable for PrincipalKey {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(bytes.into_owned())
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: 29,
        is_fixed_size: false,
    };
}

impl From<Account> for AccountKey {
    fn from(value: Account) -> Self {
        let owner = value.owner.as_slice();
        let mut bytes = Vec::with_capacity(1 + owner.len() + 32);
        bytes.push(1); // ICRC account protocol tag
        bytes.push(owner.len() as u8);
        bytes.extend_from_slice(owner);
        bytes.extend_from_slice(value.effective_subaccount());
        Self(bytes)
    }
}

impl AccountKey {
    fn icp(account_identifier: [u8; 32]) -> Self {
        let mut bytes = Vec::with_capacity(33);
        bytes.push(0); // canonical legacy ICP account protocol tag
        bytes.extend_from_slice(&account_identifier);
        Self(bytes)
    }
}

impl Storable for AccountKey {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(bytes.into_owned())
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: 63,
        is_fixed_size: false,
    };
}

macro_rules! candid_value {
    ($name:ident, $inner:ty, $max:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct $name($inner);
        impl Storable for $name {
            fn to_bytes(&self) -> Cow<'_, [u8]> {
                Cow::Owned(encode_one(&self.0).expect("encode stable value"))
            }
            fn into_bytes(self) -> Vec<u8> {
                encode_one(&self.0).expect("encode stable value")
            }
            fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
                Self(decode_one(bytes.as_ref()).expect("decode stable value"))
            }
            const BOUND: Bound = Bound::Bounded {
                max_size: $max,
                is_fixed_size: false,
            };
        }
    };
}

candid_value!(SubscriptionValue, Subscription, 256);
candid_value!(InstanceConfigValue, Option<InstanceConfig>, 256);
candid_value!(MetadataValue, Metadata, 256);
candid_value!(ObservationValue, Observation, 128);
candid_value!(PricingStateValue, PricingState, 512);
candid_value!(FundingStateValue, FundingState, 512);
candid_value!(SurplusPolicyStateValue, SurplusPolicyState, 256);
#[cfg(feature = "debug_api")]
candid_value!(DebugConfigValue, RuntimeConfig, 256);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static META: RefCell<Option<StableCell<MetadataValue, Memory>>> = const { RefCell::new(None) };
    static SUBSCRIPTIONS: RefCell<Option<StableBTreeMap<AccountKey, SubscriptionValue, Memory>>> = const { RefCell::new(None) };
    static INSTANCE_CONFIG: RefCell<Option<StableCell<InstanceConfigValue, Memory>>> = const { RefCell::new(None) };
    static GLOBAL_SUBSCRIBERS: RefCell<Option<StableBTreeMap<PrincipalKey, u8, Memory>>> = const { RefCell::new(None) };
    static PRICE_OBSERVATIONS: RefCell<Option<StableBTreeMap<u64, ObservationValue, Memory>>> = const { RefCell::new(None) };
    static PRICING_STATE: RefCell<Option<StableCell<PricingStateValue, Memory>>> = const { RefCell::new(None) };
    static FUNDING_STATE: RefCell<Option<StableCell<FundingStateValue, Memory>>> = const { RefCell::new(None) };
    static SURPLUS_POLICY_STATE: RefCell<Option<StableCell<SurplusPolicyStateValue, Memory>>> = const { RefCell::new(None) };
    #[cfg(feature = "debug_api")]
    static DEBUG_CONFIG: RefCell<Option<StableCell<DebugConfigValue, Memory>>> = const { RefCell::new(None) };
}

fn with_instance_config<R>(f: impl FnOnce(&mut StableCell<InstanceConfigValue, Memory>) -> R) -> R {
    INSTANCE_CONFIG.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(m.get(INSTANCE_CONFIG_MEMORY_ID), InstanceConfigValue(None))
            })
        });
        f(cell)
    })
}

fn with_meta<R>(f: impl FnOnce(&mut StableCell<MetadataValue, Memory>) -> R) -> R {
    META.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(m.get(META_MEMORY_ID), MetadataValue(Metadata::default()))
            })
        });
        f(cell)
    })
}

fn with_subscriptions<R>(
    f: impl FnOnce(&mut StableBTreeMap<AccountKey, SubscriptionValue, Memory>) -> R,
) -> R {
    SUBSCRIPTIONS.with_borrow_mut(|slot| {
        let map = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| StableBTreeMap::init(m.get(SUBSCRIPTIONS_MEMORY_ID)))
        });
        f(map)
    })
}

fn with_global_subscribers<R>(
    f: impl FnOnce(&mut StableBTreeMap<PrincipalKey, u8, Memory>) -> R,
) -> R {
    GLOBAL_SUBSCRIBERS.with_borrow_mut(|slot| {
        let map = slot.get_or_insert_with(|| {
            MEMORY_MANAGER
                .with_borrow(|m| StableBTreeMap::init(m.get(GLOBAL_SUBSCRIBERS_MEMORY_ID)))
        });
        f(map)
    })
}

fn with_price_observations<R>(
    f: impl FnOnce(&mut StableBTreeMap<u64, ObservationValue, Memory>) -> R,
) -> R {
    PRICE_OBSERVATIONS.with_borrow_mut(|slot| {
        let map = slot.get_or_insert_with(|| {
            MEMORY_MANAGER
                .with_borrow(|m| StableBTreeMap::init(m.get(PRICE_OBSERVATIONS_MEMORY_ID)))
        });
        f(map)
    })
}

fn with_pricing_state<R>(f: impl FnOnce(&mut StableCell<PricingStateValue, Memory>) -> R) -> R {
    PRICING_STATE.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(
                    m.get(PRICING_STATE_MEMORY_ID),
                    PricingStateValue(PricingState::default()),
                )
            })
        });
        f(cell)
    })
}

fn with_funding<R>(f: impl FnOnce(&mut StableCell<FundingStateValue, Memory>) -> R) -> R {
    FUNDING_STATE.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(
                    m.get(FUNDING_MEMORY_ID),
                    FundingStateValue(FundingState::Idle),
                )
            })
        });
        f(cell)
    })
}

fn with_surplus_policy<R>(
    f: impl FnOnce(&mut StableCell<SurplusPolicyStateValue, Memory>) -> R,
) -> R {
    SURPLUS_POLICY_STATE.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(
                    m.get(SURPLUS_POLICY_MEMORY_ID),
                    SurplusPolicyStateValue(SurplusPolicyState::default()),
                )
            })
        });
        f(cell)
    })
}

pub fn initialize_if_needed() {
    with_meta(|_| ());
    with_subscriptions(|_| ());
    with_instance_config(|_| ());
    with_global_subscribers(|_| ());
    with_price_observations(|_| ());
    with_pricing_state(|_| ());
    with_funding(|_| ());
    with_surplus_policy(|_| ());
    #[cfg(feature = "debug_api")]
    with_debug_config(|_| ());
}

pub fn initialize_instance_config(config: InstanceConfig) {
    with_instance_config(|cell| {
        assert!(
            cell.get().0.is_none(),
            "instance configuration already initialized"
        );
        cell.set(InstanceConfigValue(Some(config)));
    });
}

pub fn read_instance_config() -> InstanceConfig {
    with_instance_config(|cell| cell.get().0.clone())
        .expect("instance configuration is not initialized")
}

pub fn write_observed_profile(profile: crate::instance::ObservedLedgerProfile) {
    let mut config = read_instance_config();
    if config.observed_profile.is_none() {
        config.observed_profile = Some(profile);
        with_instance_config(|cell| cell.set(InstanceConfigValue(Some(config))));
    }
}

pub fn read_metadata() -> Metadata {
    with_meta(|cell| cell.get().0.clone())
}
pub fn write_metadata(metadata: Metadata) {
    with_meta(|cell| {
        cell.set(MetadataValue(metadata));
    });
}

pub fn modify_metadata(f: impl FnOnce(&mut Metadata)) {
    let mut meta = read_metadata();
    f(&mut meta);
    write_metadata(meta);
}

pub fn put_subscription(subscription: Subscription) {
    let runtime = crate::config::runtime();
    let key = if runtime.observed_ledger == runtime.icp_ledger {
        AccountKey::icp(account_identifier_bytes(
            subscription.subscriber,
            numbered_subaccount(subscription.numbered_subaccount),
        ))
    } else {
        AccountKey::from(subscription.account())
    };
    with_subscriptions(|map| {
        let merged =
            crate::subscription::merge_subscription(map.get(&key).map(|v| v.0), subscription);
        map.insert(key, SubscriptionValue(merged));
    });
}

pub fn get_subscription(account: Account) -> Option<Subscription> {
    with_subscriptions(|map| map.get(&AccountKey::from(account)).map(|v| v.0))
}

pub fn get_icp_subscription(account_identifier: [u8; 32]) -> Option<Subscription> {
    with_subscriptions(|map| map.get(&AccountKey::icp(account_identifier)).map(|v| v.0))
}

#[cfg(feature = "debug_api")]
pub fn get_numbered_subscription(
    subscriber: candid::Principal,
    numbered: u8,
) -> Option<Subscription> {
    let runtime = crate::config::runtime();
    if runtime.observed_ledger == runtime.icp_ledger {
        get_icp_subscription(account_identifier_bytes(
            subscriber,
            numbered_subaccount(numbered),
        ))
    } else {
        get_subscription(Account {
            owner: subscriber,
            subaccount: Some(numbered_subaccount(numbered)),
        })
    }
}

pub fn subscription_count() -> u64 {
    with_subscriptions(|map| map.len())
}

pub fn read_funding_state() -> FundingState {
    with_funding(|cell| cell.get().0.clone())
}

pub fn write_funding_state(value: FundingState) {
    with_funding(|cell| {
        cell.set(FundingStateValue(value));
    });
}

pub fn read_surplus_policy() -> SurplusPolicyState {
    with_surplus_policy(|cell| cell.get().0.clone())
}

pub fn write_surplus_policy(value: SurplusPolicyState) {
    with_surplus_policy(|cell| {
        cell.set(SurplusPolicyStateValue(value));
    });
}

pub fn put_global_subscriber(principal: candid::Principal) {
    with_global_subscribers(|map| {
        map.insert(PrincipalKey::from(principal), 1);
    });
}

pub fn global_subscribers() -> Vec<candid::Principal> {
    with_global_subscribers(|map| {
        map.iter()
            .map(|entry| candid::Principal::from_slice(&entry.key().0))
            .collect()
    })
}

#[cfg(feature = "debug_api")]
pub fn has_global_subscriber(principal: candid::Principal) -> bool {
    with_global_subscribers(|map| map.contains_key(&PrincipalKey::from(principal)))
}

#[cfg(feature = "debug_api")]
pub fn global_subscription_count() -> u64 {
    with_global_subscribers(|map| map.len())
}

pub fn read_pricing_state() -> PricingState {
    with_pricing_state(|cell| cell.get().0.clone())
}

pub fn write_pricing_state(value: PricingState) {
    with_pricing_state(|cell| {
        cell.set(PricingStateValue(value));
    });
}

pub fn price_observation(day: u64) -> Option<Observation> {
    with_price_observations(|map| map.get(&day).map(|value| value.0))
}

pub fn put_price_observation(observation: Observation) {
    with_price_observations(|map| {
        map.insert(observation.utc_day, ObservationValue(observation));
    });
}

pub fn prune_price_observations(oldest_day: u64) {
    with_price_observations(|map| {
        let expired: Vec<u64> = map
            .iter()
            .map(|entry| *entry.key())
            .take_while(|day| *day < oldest_day)
            .collect();
        for day in expired {
            map.remove(&day);
        }
    });
}

pub fn price_observations() -> Vec<Observation> {
    with_price_observations(|map| map.iter().map(|entry| entry.value().0).collect())
}

#[cfg(feature = "debug_api")]
fn with_debug_config<R>(f: impl FnOnce(&mut StableCell<DebugConfigValue, Memory>) -> R) -> R {
    DEBUG_CONFIG.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(
                    m.get(DEBUG_CONFIG_MEMORY_ID),
                    DebugConfigValue(RuntimeConfig::production(
                        candid::Principal::from_text(crate::config::ICP_LEDGER_CANISTER)
                            .expect("valid ICP Ledger principal"),
                    )),
                )
            })
        });
        f(cell)
    })
}

#[cfg(feature = "debug_api")]
pub fn write_debug_config(config: RuntimeConfig) {
    with_debug_config(|cell| {
        cell.set(DebugConfigValue(config));
    });
}

#[cfg(feature = "debug_api")]
pub fn read_debug_config() -> RuntimeConfig {
    with_debug_config(|cell| cell.get().0.clone())
}

#[cfg(test)]
mod account_key_tests {
    use super::*;

    #[test]
    fn protocol_tags_separate_icp_and_icrc_keys() {
        let owner = candid::Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap();
        let subaccount = numbered_subaccount(7);
        let icp = AccountKey::icp(account_identifier_bytes(owner, subaccount));
        let icrc = AccountKey::from(Account {
            owner,
            subaccount: Some(subaccount),
        });
        assert_eq!(icp.0[0], 0);
        assert_eq!(icp.0.len(), 33);
        assert_eq!(icrc.0[0], 1);
        assert_eq!(icrc.0.len(), 2 + owner.as_slice().len() + 32);
        assert_ne!(icp, icrc);
    }

    #[test]
    fn absent_and_zero_icrc_subaccounts_have_the_same_key() {
        let owner = candid::Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap();
        assert_eq!(
            AccountKey::from(Account {
                owner,
                subaccount: None
            }),
            AccountKey::from(Account {
                owner,
                subaccount: Some([0; 32])
            })
        );
    }
}
