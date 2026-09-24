use std::{borrow::Cow, cell::RefCell};

use candid::{decode_one, encode_one, CandidType};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    DefaultMemoryImpl, StableBTreeMap, StableCell, Storable,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug_api")]
use crate::config::RuntimeConfig;
use crate::{
    cadence::PollingMode,
    pricing::{Observation, PricingState},
    subscription::Subscription,
    surplus::SurplusPolicyState,
};

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Stable-memory IDs are part of the long-lived storage contract. Never reuse them.
const META_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBSCRIPTIONS_MEMORY_ID: MemoryId = MemoryId::new(1);
const CMC_MEMORY_ID: MemoryId = MemoryId::new(2);
#[cfg(feature = "debug_api")]
const DEBUG_CONFIG_MEMORY_ID: MemoryId = MemoryId::new(3);
const GLOBAL_SUBSCRIBERS_MEMORY_ID: MemoryId = MemoryId::new(4);
const PRICE_OBSERVATIONS_MEMORY_ID: MemoryId = MemoryId::new(5);
const PRICING_STATE_MEMORY_ID: MemoryId = MemoryId::new(6);
const FUNDING_V2_MEMORY_ID: MemoryId = MemoryId::new(7);
const SURPLUS_POLICY_MEMORY_ID: MemoryId = MemoryId::new(8);

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub bootstrapped: bool,
    pub next_block: u64,
    pub polling_mode: PollingMode,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            bootstrapped: false,
            next_block: 0,
            polling_mode: PollingMode::ReserveProtection,
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum CmcState {
    #[default]
    Idle,
    TransferPending {
        amount_e8s: u64,
        fee_e8s: u64,
        created_at_time_nanos: u64,
    },
    NotifyPending {
        block_index: u64,
    },
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum FundingStateV2 {
    #[default]
    Uninitialized,
    Idle,
    CmcTransferPending {
        amount_e8s: u64,
        fee_e8s: u64,
        created_at_time_nanos: u64,
        planned_surplus_e8s: u64,
        planned_surplus_fee_e8s: u64,
    },
    CmcNotifyPending {
        block_index: u64,
        planned_surplus_e8s: u64,
        planned_surplus_fee_e8s: u64,
    },
    SurplusTransferPending {
        amount_e8s: u64,
        fee_e8s: u64,
        created_at_time_nanos: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AccountKey([u8; 32]);

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

impl From<[u8; 32]> for AccountKey {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl Storable for AccountKey {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }
    fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let mut out = [0u8; 32];
        out.copy_from_slice(bytes.as_ref());
        Self(out)
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: 32,
        is_fixed_size: true,
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

candid_value!(SubscriptionValue, Subscription, 128);
candid_value!(MetadataValue, Metadata, 256);
candid_value!(CmcStateValue, CmcState, 256);
candid_value!(ObservationValue, Observation, 128);
candid_value!(PricingStateValue, PricingState, 512);
candid_value!(FundingStateV2Value, FundingStateV2, 512);
candid_value!(SurplusPolicyStateValue, SurplusPolicyState, 256);
#[cfg(feature = "debug_api")]
candid_value!(DebugConfigValue, RuntimeConfig, 256);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static META: RefCell<Option<StableCell<MetadataValue, Memory>>> = const { RefCell::new(None) };
    static SUBSCRIPTIONS: RefCell<Option<StableBTreeMap<AccountKey, SubscriptionValue, Memory>>> = const { RefCell::new(None) };
    static CMC_STATE: RefCell<Option<StableCell<CmcStateValue, Memory>>> = const { RefCell::new(None) };
    static GLOBAL_SUBSCRIBERS: RefCell<Option<StableBTreeMap<PrincipalKey, u8, Memory>>> = const { RefCell::new(None) };
    static PRICE_OBSERVATIONS: RefCell<Option<StableBTreeMap<u64, ObservationValue, Memory>>> = const { RefCell::new(None) };
    static PRICING_STATE: RefCell<Option<StableCell<PricingStateValue, Memory>>> = const { RefCell::new(None) };
    static FUNDING_V2_STATE: RefCell<Option<StableCell<FundingStateV2Value, Memory>>> = const { RefCell::new(None) };
    static SURPLUS_POLICY_STATE: RefCell<Option<StableCell<SurplusPolicyStateValue, Memory>>> = const { RefCell::new(None) };
    #[cfg(feature = "debug_api")]
    static DEBUG_CONFIG: RefCell<Option<StableCell<DebugConfigValue, Memory>>> = const { RefCell::new(None) };
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

fn with_cmc<R>(f: impl FnOnce(&mut StableCell<CmcStateValue, Memory>) -> R) -> R {
    CMC_STATE.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(m.get(CMC_MEMORY_ID), CmcStateValue(CmcState::Idle))
            })
        });
        f(cell)
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

fn with_funding_v2<R>(f: impl FnOnce(&mut StableCell<FundingStateV2Value, Memory>) -> R) -> R {
    FUNDING_V2_STATE.with_borrow_mut(|slot| {
        let cell = slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(
                    m.get(FUNDING_V2_MEMORY_ID),
                    FundingStateV2Value(FundingStateV2::Uninitialized),
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
    with_cmc(|_| ());
    with_global_subscribers(|_| ());
    with_price_observations(|_| ());
    with_pricing_state(|_| ());
    with_funding_v2(|_| ());
    with_surplus_policy(|_| ());
    #[cfg(feature = "debug_api")]
    with_debug_config(|_| ());
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
    let key = AccountKey(subscription.account_identifier());
    with_subscriptions(|map| {
        let merged =
            crate::subscription::merge_subscription(map.get(&key).map(|v| v.0), subscription);
        map.insert(key, SubscriptionValue(merged));
    });
}

pub fn get_subscription(account_identifier: [u8; 32]) -> Option<Subscription> {
    with_subscriptions(|map| map.get(&AccountKey(account_identifier)).map(|v| v.0))
}

#[cfg(feature = "debug_api")]
pub fn subscription_count() -> u64 {
    with_subscriptions(|map| map.len())
}

pub fn read_cmc_state() -> CmcState {
    with_cmc(|cell| cell.get().0.clone())
}
pub fn read_funding_state() -> FundingStateV2 {
    let current = with_funding_v2(|cell| cell.get().0.clone());
    if current != FundingStateV2::Uninitialized {
        return current;
    }
    let migrated = match read_cmc_state() {
        CmcState::Idle => FundingStateV2::Idle,
        CmcState::TransferPending {
            amount_e8s,
            fee_e8s,
            created_at_time_nanos,
        } => FundingStateV2::CmcTransferPending {
            amount_e8s,
            fee_e8s,
            created_at_time_nanos,
            planned_surplus_e8s: 0,
            planned_surplus_fee_e8s: 0,
        },
        CmcState::NotifyPending { block_index } => FundingStateV2::CmcNotifyPending {
            block_index,
            planned_surplus_e8s: 0,
            planned_surplus_fee_e8s: 0,
        },
    };
    write_funding_state(migrated.clone());
    migrated
}

pub fn write_funding_state(value: FundingStateV2) {
    with_funding_v2(|cell| {
        cell.set(FundingStateV2Value(value));
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
                    DebugConfigValue(RuntimeConfig::production()),
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
