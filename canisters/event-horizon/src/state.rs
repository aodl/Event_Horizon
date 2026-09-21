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
use crate::{cadence::PollingMode, subscription::Subscription};

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Stable-memory IDs are part of the long-lived storage contract. Never reuse them.
const META_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBSCRIPTIONS_MEMORY_ID: MemoryId = MemoryId::new(1);
const CMC_MEMORY_ID: MemoryId = MemoryId::new(2);
#[cfg(feature = "debug_api")]
const DEBUG_CONFIG_MEMORY_ID: MemoryId = MemoryId::new(3);

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AccountKey([u8; 32]);

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
#[cfg(feature = "debug_api")]
candid_value!(DebugConfigValue, RuntimeConfig, 256);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static META: RefCell<Option<StableCell<MetadataValue, Memory>>> = const { RefCell::new(None) };
    static SUBSCRIPTIONS: RefCell<Option<StableBTreeMap<AccountKey, SubscriptionValue, Memory>>> = const { RefCell::new(None) };
    static CMC_STATE: RefCell<Option<StableCell<CmcStateValue, Memory>>> = const { RefCell::new(None) };
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

pub fn initialize_if_needed() {
    with_meta(|_| ());
    with_subscriptions(|_| ());
    with_cmc(|_| ());
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
pub fn write_cmc_state(value: CmcState) {
    with_cmc(|cell| {
        cell.set(CmcStateValue(value));
    });
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
