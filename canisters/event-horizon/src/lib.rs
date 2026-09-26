//! Event Horizon backend.
//!
//! Production exposes only the bounded `get_instance` and `get_pricing` queries.
//! Autonomous timers drive the ledger readers and ICP-to-cycles maintenance. The
//! `debug_api` feature exists only for local/PocketIC validation and must not be present
//! in the canonical production Wasm.

mod account;
mod cadence;
mod clients;
mod config;
mod funding;
mod instance;
mod logging;
mod memo;
mod polling;
mod pricing;
mod scheduler;
mod state;
mod subscription;
mod surplus;

#[cfg(feature = "debug_api")]
use candid::Principal;

#[cfg(feature = "debug_api")]
use debug::{DebugCursorArgs, DebugInitArgs, DebugState, DebugSubscriptionArgs};

pub use account::{account_identifier_bytes, numbered_subaccount};
pub use cadence::{next_mode, PollingMode};
pub use instance::{InitArgs, InstanceInfo, ObservedLedgerProfile};
pub use memo::{parse_subscription_memo, MemoParseError, SubscriptionDeclaration};
pub use pricing::{Price, Pricing, PublicPrice};
pub use subscription::{merge_subscription, Subscription};

#[cfg(not(feature = "debug_api"))]
#[ic_cdk::init]
fn init(args: InitArgs) {
    instance::validate_observed_ledger(args.observed_ledger);
    state::initialize_if_needed();
    state::initialize_instance_config(instance::InstanceConfig {
        observed_ledger: args.observed_ledger,
        observed_profile: None,
    });
    instance::log_config();
    scheduler::start();
}

#[cfg(not(feature = "debug_api"))]
#[ic_cdk::post_upgrade]
fn post_upgrade() {
    state::initialize_if_needed();
    scheduler::start();
}

#[ic_cdk::query]
fn get_pricing() -> Pricing {
    pricing::get_pricing()
}

#[ic_cdk::query]
fn get_instance() -> InstanceInfo {
    instance::get_instance()
}

#[cfg(feature = "debug_api")]
mod debug {
    use super::*;
    use candid::{CandidType, Principal};
    use serde::Deserialize;

    #[derive(CandidType, Deserialize)]
    pub struct DebugInitArgs {
        pub observed_ledger: Principal,
        pub icp_ledger: Principal,
        pub cmc_canister: Principal,
        pub historian_canister: Principal,
        pub faucet_canister: Principal,
        pub surplus_canister: Option<Principal>,
    }

    impl From<DebugInitArgs> for config::RuntimeConfig {
        fn from(value: DebugInitArgs) -> Self {
            Self {
                observed_ledger: value.observed_ledger,
                icp_ledger: value.icp_ledger,
                cmc_canister: value.cmc_canister,
                historian_canister: value.historian_canister,
                faucet_canister: value.faucet_canister,
                surplus_canister: value.surplus_canister,
            }
        }
    }

    #[ic_cdk::init]
    fn init(args: DebugInitArgs) {
        state::initialize_if_needed();
        instance::validate_observed_ledger(args.observed_ledger);
        state::initialize_instance_config(instance::InstanceConfig {
            observed_ledger: args.observed_ledger,
            observed_profile: None,
        });
        state::write_debug_config(args.into());
        // Debug builds are manually driven to keep PocketIC tests deterministic.
    }

    #[ic_cdk::post_upgrade]
    fn post_upgrade() {
        state::initialize_if_needed();
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugState {
        pub admission_bootstrapped: bool,
        pub admission_next_block: u64,
        pub observed_bootstrapped: bool,
        pub observed_next_block: u64,
        pub polling_mode: PollingMode,
        pub subscriptions: u64,
        pub global_subscriptions: u64,
        pub cmc_state: String,
        pub surplus_policy: String,
        pub surplus_destination: Option<Principal>,
        pub pricing: Pricing,
    }

    #[ic_cdk::query]
    fn debug_state() -> DebugState {
        let meta = state::read_metadata();
        DebugState {
            admission_bootstrapped: meta.admission_bootstrapped,
            admission_next_block: meta.admission_next_block,
            observed_bootstrapped: meta.observed_bootstrapped,
            observed_next_block: meta.observed_next_block,
            polling_mode: meta.polling_mode,
            subscriptions: state::subscription_count(),
            global_subscriptions: state::global_subscription_count(),
            cmc_state: format!("{:?}", state::read_funding_state()),
            surplus_policy: format!("{:?}", state::read_surplus_policy()),
            surplus_destination: config::runtime().surplus_canister,
            pricing: pricing::get_pricing(),
        }
    }

    #[ic_cdk::update]
    async fn debug_poll_once() {
        scheduler::debug_poll_once().await;
    }

    #[ic_cdk::update]
    async fn debug_funding_once() {
        scheduler::debug_funding_once().await;
    }

    #[ic_cdk::update]
    async fn debug_pricing_once() {
        scheduler::debug_pricing_once().await;
    }

    #[ic_cdk::update]
    fn debug_set_surplus_level(level: u8) {
        assert!(level <= config::SURPLUS_MAX_LEVEL, "invalid surplus level");
        state::write_surplus_policy(surplus::SurplusPolicyState {
            initialized: true,
            epoch_started_at_seconds: ic_cdk::api::time() / 1_000_000_000,
            epoch_min_liquid_cycles: ic_cdk::api::canister_liquid_cycle_balance(),
            diversion_level: level,
        });
    }

    #[ic_cdk::update]
    fn debug_set_surplus_canister(surplus_canister: Option<Principal>) {
        let mut runtime = config::runtime();
        runtime.surplus_canister = surplus_canister;
        state::write_debug_config(runtime);
    }

    #[ic_cdk::update]
    fn debug_set_liquid_cycles_override(value: Option<u128>) {
        funding::debug_set_liquid_cycles_override(value);
    }

    #[ic_cdk::update]
    fn debug_set_ledger_canister(ledger_canister: Principal) {
        let mut runtime = config::runtime();
        runtime.icp_ledger = ledger_canister;
        state::write_debug_config(runtime);
    }

    #[ic_cdk::update]
    fn debug_start_schedulers() {
        scheduler::debug_start();
    }

    #[ic_cdk::query]
    fn debug_timer_count() -> u8 {
        scheduler::debug_timer_count()
    }

    #[ic_cdk::query]
    fn debug_icrc3_get_blocks_calls() -> u64 {
        clients::icrc3::debug_get_blocks_calls()
    }

    #[ic_cdk::query]
    fn debug_legacy_query_blocks_calls() -> u64 {
        clients::icp_ledger::debug_query_blocks_calls()
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugSubscriptionArgs {
        subscriber: Principal,
        subaccount: u8,
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugCursorArgs {
        admission_bootstrapped: bool,
        admission_next_block: u64,
        observed_bootstrapped: bool,
        observed_next_block: u64,
    }

    #[ic_cdk::update]
    fn debug_set_cursors(args: DebugCursorArgs) {
        state::modify_metadata(|m| {
            m.admission_bootstrapped = args.admission_bootstrapped;
            m.admission_next_block = args.admission_next_block;
            m.observed_bootstrapped = args.observed_bootstrapped;
            m.observed_next_block = args.observed_next_block;
        });
    }

    #[ic_cdk::query]
    fn debug_subscription(args: DebugSubscriptionArgs) -> Option<Subscription> {
        state::get_numbered_subscription(args.subscriber, args.subaccount)
    }

    #[ic_cdk::query]
    fn debug_global_subscription(subscriber: Principal) -> bool {
        state::has_global_subscriber(subscriber)
    }
}

ic_cdk::export_candid!();
