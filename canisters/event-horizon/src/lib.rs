//! Event Horizon backend.
//!
//! Production intentionally exposes no application methods. Autonomous timers drive the
//! Ledger reader and ICP-to-cycles maintenance. The `debug_api` feature exists only for
//! local/PocketIC validation and must not be present in the canonical production Wasm.

mod account;
mod cadence;
mod clients;
mod config;
mod funding;
mod logging;
mod memo;
mod polling;
mod pricing;
mod scheduler;
mod state;
mod subscription;

#[cfg(feature = "debug_api")]
use candid::Principal;

#[cfg(feature = "debug_api")]
use debug::{DebugInitArgs, DebugState, DebugSubscriptionArgs};

pub use account::{account_identifier_bytes, numbered_subaccount};
pub use cadence::{next_mode, PollingMode};
pub use memo::{parse_subscription_memo, MemoParseError, SubscriptionDeclaration};
pub use pricing::{Price, Pricing};
pub use subscription::{merge_subscription, Subscription};

#[cfg(not(feature = "debug_api"))]
#[ic_cdk::init]
fn init() {
    state::initialize_if_needed();
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

#[cfg(feature = "debug_api")]
mod debug {
    use super::*;
    use candid::{CandidType, Principal};
    use serde::Deserialize;

    #[derive(CandidType, Deserialize)]
    pub struct DebugInitArgs {
        pub ledger_canister: Principal,
        pub cmc_canister: Principal,
        pub historian_canister: Principal,
        pub faucet_canister: Principal,
    }

    impl From<DebugInitArgs> for config::RuntimeConfig {
        fn from(value: DebugInitArgs) -> Self {
            Self {
                ledger_canister: value.ledger_canister,
                cmc_canister: value.cmc_canister,
                historian_canister: value.historian_canister,
                faucet_canister: value.faucet_canister,
            }
        }
    }

    #[ic_cdk::init]
    fn init(args: DebugInitArgs) {
        state::initialize_if_needed();
        state::write_debug_config(args.into());
        // Debug builds are manually driven to keep PocketIC tests deterministic.
    }

    #[ic_cdk::post_upgrade]
    fn post_upgrade() {
        state::initialize_if_needed();
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugState {
        pub bootstrapped: bool,
        pub next_block: u64,
        pub polling_mode: PollingMode,
        pub subscriptions: u64,
        pub global_subscriptions: u64,
        pub cmc_state: String,
        pub pricing: Pricing,
    }

    #[ic_cdk::query]
    fn debug_state() -> DebugState {
        let meta = state::read_metadata();
        DebugState {
            bootstrapped: meta.bootstrapped,
            next_block: meta.next_block,
            polling_mode: meta.polling_mode,
            subscriptions: state::subscription_count(),
            global_subscriptions: state::global_subscription_count(),
            cmc_state: format!("{:?}", state::read_cmc_state()),
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
    fn debug_start_schedulers() {
        scheduler::debug_start();
    }

    #[ic_cdk::query]
    fn debug_timer_count() -> u8 {
        scheduler::debug_timer_count()
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugSubscriptionArgs {
        subscriber: Principal,
        subaccount: u8,
    }

    #[ic_cdk::query]
    fn debug_subscription(args: DebugSubscriptionArgs) -> Option<Subscription> {
        let account =
            account_identifier_bytes(args.subscriber, numbered_subaccount(args.subaccount));
        state::get_subscription(account)
    }

    #[ic_cdk::query]
    fn debug_global_subscription(subscriber: Principal) -> bool {
        state::has_global_subscriber(subscriber)
    }
}

ic_cdk::export_candid!();
