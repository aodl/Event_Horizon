use std::{borrow::Cow, cell::RefCell};

use candid::{decode_one, encode_one, CandidType};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    DefaultMemoryImpl, StableBTreeMap, StableCell, Storable,
};
use serde::{Deserialize, Serialize};

use crate::account::account_identifier_bytes;
#[cfg(feature = "debug_api")]
use crate::config::RuntimeConfig;
use crate::instance::InstanceConfig;
use crate::{
    cadence::PollingMode,
    pricing::{Observation, PricingState},
    subscription::{Subscription, WatchTarget},
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
        let mut bytes = Vec::with_capacity(2 + owner.len() + 32);
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
    with_aliases(|_| ());
    with_alias_metadata(|_| ());
    with_external(|_| ());
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

#[cfg(any(test, feature = "debug_api"))]
pub fn put_subscription(subscription: Subscription) {
    put_relation(crate::subscription::SubjectSubscription::self_subject(
        subscription,
    ));
}

pub fn get_subscription(account: Account) -> Option<Subscription> {
    with_subscriptions(|map| map.get(&AccountKey::from(account)).map(|v| v.0))
}

pub fn get_icp_subscription(account_identifier: [u8; 32]) -> Option<Subscription> {
    with_subscriptions(|map| map.get(&AccountKey::icp(account_identifier)).map(|v| v.0))
}

#[cfg(feature = "debug_api")]
pub fn get_target_subscription(
    subscriber: candid::Principal,
    target: WatchTarget,
) -> Option<Subscription> {
    let runtime = crate::config::runtime();
    let governance = read_instance_config()
        .observed_profile
        .and_then(|p| p.neuron_governance);
    let account = Subscription {
        subscriber,
        target,
        minimum_units: 0u8.into(),
    }
    .account(governance)?;
    if runtime.observed_ledger == runtime.icp_ledger {
        get_icp_subscription(account_identifier_bytes(
            account.owner,
            *account.effective_subaccount(),
        ))
    } else {
        get_subscription(account)
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

#[cfg(test)]
thread_local! { static TEST_WRITES: std::cell::Cell<[u64;4]> = const { std::cell::Cell::new([0;4]) }; }
#[cfg(test)]
fn test_written(kind: usize) {
    TEST_WRITES.with(|v| {
        let mut n = v.get();
        n[kind] += 1;
        v.set(n);
    });
}
#[cfg(test)]
pub(crate) fn test_stable_writes() -> [u64; 4] {
    TEST_WRITES.with(std::cell::Cell::get)
}

pub fn put_global_subscriber(principal: candid::Principal) {
    with_global_subscribers(|map| {
        let key = PrincipalKey::from(principal);
        if !map.contains_key(&key) {
            map.insert(key, 1);
            #[cfg(test)]
            test_written(2);
        }
    });
}

#[cfg(test)]
thread_local! { static TEST_GLOBAL_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(crate) fn test_global_visits() -> u64 {
    TEST_GLOBAL_VISITS.with(std::cell::Cell::get)
}

pub fn global_subscribers() -> Vec<candid::Principal> {
    with_global_subscribers(|map| {
        map.iter()
            .map(|entry| {
                #[cfg(test)]
                TEST_GLOBAL_VISITS.with(|n| n.set(n.get() + 1));
                candid::Principal::from_slice(&entry.key().0)
            })
            .collect()
    })
}

#[cfg(feature = "debug_api")]
pub fn has_global_subscriber(principal: candid::Principal) -> bool {
    with_global_subscribers(|map| map.contains_key(&PrincipalKey::from(principal)))
}

pub fn global_subscription_count() -> u64 {
    with_global_subscribers(|map| map.len())
}

#[cfg(feature = "debug_api")]
fn synthetic_subscriber(index: u64) -> candid::Principal {
    let mut bytes = [0u8; 9];
    bytes[0] = 0x7f;
    bytes[1..].copy_from_slice(&index.to_be_bytes());
    candid::Principal::from_slice(&bytes)
}

#[cfg(feature = "debug_api")]
pub fn debug_seed_subscribers(kind: &str, start: u64, count: u64) {
    assert!(
        count <= 128,
        "debug seed batches are limited to 128 records"
    );
    for index in start..start.saturating_add(count) {
        let subscriber = synthetic_subscriber(index);
        match kind {
            "global" => put_global_subscriber(subscriber),
            "specific" => put_subscription(Subscription {
                subscriber,
                target: WatchTarget::Subaccount(0),
                minimum_units: 0u8.into(),
            }),
            _ => ic_cdk::trap("unknown debug subscriber seed kind"),
        }
    }
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
                        None,
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
    use crate::numbered_subaccount;

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

// Additive regions only: IDs0–8 and their Storable encodings are untouched.
const ALIASES_MEMORY_ID: MemoryId = MemoryId::new(9);
const ALIAS_METADATA_MEMORY_ID: MemoryId = MemoryId::new(10);
const EXTERNAL_SUBSCRIPTIONS_MEMORY_ID: MemoryId = MemoryId::new(11);
candid_value!(AliasMetadataValue, crate::aliases::AliasMetadata, 256);
candid_value!(ExternalValue, crate::subscription::SubjectSubscription, 512);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExternalKey(Vec<u8>);
impl Storable for ExternalKey {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
    fn from_bytes(b: Cow<'_, [u8]>) -> Self {
        Self(b.into_owned())
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: 134,
        is_fixed_size: false,
    };
}
impl ExternalKey {
    fn prefix(account: &AccountKey) -> Vec<u8> {
        let mut b = vec![account.0.len() as u8];
        b.extend_from_slice(&account.0);
        b
    }
    fn new(account: &AccountKey, s: &crate::subscription::SubjectSubscription) -> Self {
        let mut b = Self::prefix(account);
        for p in [s.subscription.subscriber, s.subject] {
            b.push(p.as_slice().len() as u8);
            b.extend_from_slice(p.as_slice())
        }
        b.push(match s.subscription.target {
            WatchTarget::Subaccount(_) => 0,
            WatchTarget::NeuronNonce(_) => 1,
        });
        b.extend_from_slice(&s.subscription.target.value().to_be_bytes());
        Self(b)
    }
}
thread_local! {
    static ALIASES:RefCell<Option<StableBTreeMap<String,candid::Principal,Memory>>>=const{RefCell::new(None)};
    static ALIAS_METADATA:RefCell<Option<StableCell<AliasMetadataValue,Memory>>>=const{RefCell::new(None)};
    static EXTERNAL_SUBSCRIPTIONS:RefCell<Option<StableBTreeMap<ExternalKey,ExternalValue,Memory>>>=const{RefCell::new(None)};
}
fn with_aliases<R>(
    f: impl FnOnce(&mut StableBTreeMap<String, candid::Principal, Memory>) -> R,
) -> R {
    ALIASES.with_borrow_mut(|slot| {
        f(slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| StableBTreeMap::init(m.get(ALIASES_MEMORY_ID)))
        }))
    })
}
fn with_alias_metadata<R>(f: impl FnOnce(&mut StableCell<AliasMetadataValue, Memory>) -> R) -> R {
    ALIAS_METADATA.with_borrow_mut(|slot| {
        f(slot.get_or_insert_with(|| {
            MEMORY_MANAGER.with_borrow(|m| {
                StableCell::init(
                    m.get(ALIAS_METADATA_MEMORY_ID),
                    AliasMetadataValue(Default::default()),
                )
            })
        }))
    })
}
fn with_external<R>(
    f: impl FnOnce(&mut StableBTreeMap<ExternalKey, ExternalValue, Memory>) -> R,
) -> R {
    EXTERNAL_SUBSCRIPTIONS.with_borrow_mut(|slot| {
        f(slot.get_or_insert_with(|| {
            MEMORY_MANAGER
                .with_borrow(|m| StableBTreeMap::init(m.get(EXTERNAL_SUBSCRIPTIONS_MEMORY_ID)))
        }))
    })
}
pub fn alias(name: &str) -> Option<candid::Principal> {
    with_aliases(|m| m.get(&name.to_string()))
}
pub fn alias_table() -> std::collections::BTreeMap<String, candid::Principal> {
    with_aliases(|m| m.iter().map(|e| (e.key().clone(), e.value())).collect())
}
pub fn alias_metadata() -> crate::aliases::AliasMetadata {
    with_alias_metadata(|m| m.get().0.clone())
}
pub fn write_alias_metadata(meta: crate::aliases::AliasMetadata) {
    with_alias_metadata(|m| {
        m.set(AliasMetadataValue(meta));
    });
}
pub fn publish_aliases(
    items: Vec<crate::clients::historian::CanisterAlias>,
    revision: Vec<u8>,
    now: u64,
) {
    with_aliases(|m| {
        for i in items {
            if m.get(&i.alias) != Some(i.canister_id) {
                m.insert(i.alias, i.canister_id);
                #[cfg(test)]
                test_written(3);
            }
        }
    });
    let mut meta = alias_metadata();
    meta.revision = Some(revision);
    meta.last_success = Some(now);
    write_alias_metadata(meta);
}
/// Immutable derivation inputs for one admission operation, never mutable prices,
/// aliases, balances, or endowment evidence.
#[derive(Clone, Copy)]
pub(crate) struct RelationContext {
    shared: bool,
    governance: Option<candid::Principal>,
}
pub(crate) fn relation_context(runtime: &crate::config::RuntimeConfig) -> RelationContext {
    RelationContext {
        shared: runtime.observed_ledger == runtime.icp_ledger,
        governance: read_instance_config()
            .observed_profile
            .and_then(|p| p.neuron_governance),
    }
}
#[derive(Clone)]
pub(crate) struct RelationKey {
    account: AccountKey,
    external: Option<ExternalKey>,
}
impl RelationKey {
    pub(crate) fn icp_identifier(&self) -> Option<[u8; 32]> {
        if self.account.0.first() == Some(&0) {
            self.account.0[1..].try_into().ok()
        } else {
            None
        }
    }
}
#[cfg(test)]
thread_local! {static TEST_KEY_DERIVATIONS: std::cell::Cell<u64> = const {std::cell::Cell::new(0)};}
#[cfg(test)]
pub(crate) fn test_key_derivations() -> u64 {
    TEST_KEY_DERIVATIONS.get()
}
pub(crate) fn relation_key(
    s: &crate::subscription::SubjectSubscription,
    context: RelationContext,
) -> RelationKey {
    #[cfg(test)]
    TEST_KEY_DERIVATIONS.set(TEST_KEY_DERIVATIONS.get() + 1);
    let account = s.account(context.governance).expect("verified governance");
    let account = if context.shared {
        AccountKey::icp(account_identifier_bytes(
            account.owner,
            *account.effective_subaccount(),
        ))
    } else {
        AccountKey::from(account)
    };
    let external = (s.subject != s.subscription.subscriber).then(|| ExternalKey::new(&account, s));
    RelationKey { account, external }
}
pub(crate) fn relation_at(key: &RelationKey) -> Option<crate::subscription::SubjectSubscription> {
    match &key.external {
        Some(key) => with_external(|m| m.get(key).map(|v| v.0)),
        None => with_subscriptions(|m| {
            m.get(&key.account)
                .map(|v| crate::subscription::SubjectSubscription::self_subject(v.0))
        }),
    }
}
pub(crate) fn put_relation_at(key: RelationKey, s: crate::subscription::SubjectSubscription) {
    match key.external {
        Some(key) => with_external(|m| {
            if m.get(&key).is_some_and(|old| {
                old.0.identity() == s.identity()
                    && old.0.subscription.minimum_units <= s.subscription.minimum_units
            }) {
                return;
            }
            m.insert(key, ExternalValue(s));
            #[cfg(test)]
            test_written(1);
        }),
        None => with_subscriptions(|m| {
            let candidate = s.subscription;
            if m.get(&key.account).is_some_and(|old| {
                old.0.subscriber == candidate.subscriber
                    && old.0.target == candidate.target
                    && old.0.minimum_units <= candidate.minimum_units
            }) {
                return;
            }
            m.insert(key.account, SubscriptionValue(candidate));
            #[cfg(test)]
            test_written(0);
        }),
    }
}
#[cfg(any(test, feature = "debug_api"))]
pub fn put_relation(s: crate::subscription::SubjectSubscription) {
    let context = relation_context(&crate::config::runtime());
    let key = relation_key(&s, context);
    put_relation_at(key, s);
}
#[cfg(test)]
pub fn get_relation(
    subscriber: candid::Principal,
    subject: candid::Principal,
    target: WatchTarget,
) -> Option<crate::subscription::SubjectSubscription> {
    let s = crate::subscription::SubjectSubscription {
        subject,
        subscription: Subscription {
            subscriber,
            target,
            minimum_units: 0u8.into(),
        },
    };
    let key = relation_key(&s, relation_context(&crate::config::runtime()));
    relation_at(&key)
}

fn interested(
    account: AccountKey,
    own: Option<Subscription>,
) -> Vec<crate::subscription::SubjectSubscription> {
    let prefix = ExternalKey::prefix(&account);
    let mut out: Vec<_> = own
        .into_iter()
        .map(crate::subscription::SubjectSubscription::self_subject)
        .collect();
    with_external(|m| {
        out.extend(
            m.range(ExternalKey(prefix.clone())..)
                .take_while(|e| e.key().0.starts_with(&prefix))
                .map(|e| e.value().0),
        );
    });
    out
}
pub fn interested_icrc(account: Account) -> Vec<crate::subscription::SubjectSubscription> {
    interested(AccountKey::from(account), get_subscription(account))
}
pub fn interested_icp(account: [u8; 32]) -> Vec<crate::subscription::SubjectSubscription> {
    interested(AccountKey::icp(account), get_icp_subscription(account))
}

#[cfg(test)]
mod external_key_tests {
    use super::*;
    use crate::subscription::SubjectSubscription;
    #[test]
    fn icrc_account_key_keeps_exact_bytes_without_growing_the_buffer() {
        for length in [0, 1, 10, 29] {
            let owner = candid::Principal::from_slice(&vec![7; length]);
            for subaccount in [None, Some([0; 32]), Some([91; 32])] {
                let key = AccountKey::from(Account { owner, subaccount });
                let mut expected = vec![1, length as u8];
                expected.extend_from_slice(owner.as_slice());
                expected.extend_from_slice(&subaccount.unwrap_or([0; 32]));
                assert_eq!(key.0, expected);
                assert_eq!(key.0.capacity(), key.0.len());
                assert_eq!(AccountKey::from_bytes(key.to_bytes()).0, expected);
                assert!(key.0.len() <= 63);
            }
        }
    }

    #[test]
    fn composite_keys_isolate_protocol_account_and_relation() {
        let subject = candid::Principal::from_slice(&[2, 1]);
        let relation = SubjectSubscription {
            subject,
            subscription: Subscription {
                subscriber: candid::Principal::from_slice(&[3, 1]),
                target: WatchTarget::Subaccount(7),
                minimum_units: 0u8.into(),
            },
        };
        let icp = AccountKey::icp([0; 32]);
        let icrc = AccountKey::from(Account {
            owner: subject,
            subaccount: None,
        });
        let key = ExternalKey::new(&icp, &relation);
        assert!(key.0.starts_with(&ExternalKey::prefix(&icp)));
        assert!(!key.0.starts_with(&ExternalKey::prefix(&icrc)));
        let mut other = relation.clone();
        other.subscription.subscriber = candid::Principal::from_slice(&[4, 1]);
        assert_ne!(key, ExternalKey::new(&icp, &other));
        other = relation.clone();
        other.subject = candid::Principal::from_slice(&[5, 1]);
        assert_ne!(key, ExternalKey::new(&icp, &other));
        other = relation.clone();
        other.subscription.target = WatchTarget::NeuronNonce(7);
        assert_ne!(key, ExternalKey::new(&icp, &other));
        assert!(key.0.len() <= 134);
    }
}

#[cfg(test)]
mod unchanged_write_tests {
    use super::*;
    #[test]
    fn unchanged_effective_values_skip_map_writes_but_alias_metadata_advances() {
        std::thread::spawn(|| {
            let a = candid::Principal::from_slice(&[1, 1]);
            let b = candid::Principal::from_slice(&[2, 1]);
            initialize_instance_config(InstanceConfig {
                observed_ledger: a,
                observed_profile: None,
                sns_root: None,
                surplus_canister: None,
            });
            let sub = |minimum| Subscription {
                subscriber: a,
                target: WatchTarget::Subaccount(7),
                minimum_units: candid::Nat::from(minimum),
            };
            put_subscription(sub(100u64));
            put_subscription(sub(100u64));
            put_subscription(sub(200u64));
            put_subscription(sub(20u64));
            let external = |minimum| crate::subscription::SubjectSubscription {
                subject: b,
                subscription: sub(minimum),
            };
            put_relation(external(100u64));
            put_relation(external(100u64));
            put_relation(external(200u64));
            put_relation(external(20u64));
            assert_eq!(
                get_relation(a, a, WatchTarget::Subaccount(7))
                    .unwrap()
                    .subscription
                    .minimum_units,
                candid::Nat::from(20u8)
            );
            assert_eq!(
                get_relation(a, b, WatchTarget::Subaccount(7))
                    .unwrap()
                    .subscription
                    .minimum_units,
                candid::Nat::from(20u8)
            );
            put_global_subscriber(a);
            put_global_subscriber(a);
            let mut meta = alias_metadata();
            meta.last_attempt = Some(99);
            write_alias_metadata(meta);
            let items = || {
                vec![crate::clients::historian::CanisterAlias {
                    alias: "A".into(),
                    canister_id: a,
                }]
            };
            publish_aliases(items(), vec![1; 32], 100);
            publish_aliases(items(), vec![1; 32], 101);
            assert_eq!(test_stable_writes(), [2, 2, 1, 1]);
            assert_eq!(global_subscription_count(), 1);
            assert_eq!(alias("A"), Some(a));
            assert_eq!(alias_metadata().last_success, Some(101));
            assert_eq!(alias_metadata().last_attempt, Some(99));
            assert_eq!(alias_metadata().revision, Some(vec![1; 32]));
        })
        .join()
        .unwrap();
    }
}
