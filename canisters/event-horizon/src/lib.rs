//! Event Horizon backend.
//!
//! Production exposes three read-only queries and one authenticated poll wake.
//! Autonomous timers drive the ledger readers and ICP-to-cycles maintenance. The
//! `debug_api` feature exists only for local/PocketIC validation and must not be present
//! in the canonical production Wasm.

mod account;
mod aliases;
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

use candid::Principal;

#[cfg(feature = "debug_api")]
use debug::{
    DebugCursorArgs, DebugExternalSeed, DebugInitArgs, DebugSeedSubscribers, DebugState,
    DebugSubscriptionArgs,
};

pub use account::{account_identifier_bytes, neuron_staking_subaccount, numbered_subaccount};
pub use cadence::{next_mode, PollingMode};
pub use instance::{InitArgs, InstanceInfo, ObservedLedgerProfile};
pub use memo::{parse_subscription_memo, MemoParseError, SubscriptionDeclaration};
pub use pricing::{Price, Pricing, PublicPrice};
pub use subscription::{merge_subscription, Subscription, WatchTarget};

#[cfg(not(feature = "debug_api"))]
#[ic_cdk::init]
fn init(args: InitArgs) {
    let icp_ledger =
        config::RuntimeConfig::production(args.observed_ledger, args.surplus_canister).icp_ledger;
    instance::validate_init(
        args.observed_ledger,
        args.sns_root,
        args.surplus_canister,
        args.icp_admission_notifier,
        icp_ledger,
        ic_cdk::api::canister_self(),
    );
    state::initialize_if_needed();
    configure_poke_callers(
        args.allowed_poke_callers,
        args.observed_ledger == icp_ledger,
    );
    state::initialize_instance_config(instance::InstanceConfig {
        observed_ledger: args.observed_ledger,
        observed_profile: None,
        sns_root: args.sns_root,
        surplus_canister: args.surplus_canister,
        icp_admission_notifier: args.icp_admission_notifier,
    });
    instance::log_config();
    scheduler::start();
    aliases::start();
}

#[cfg(not(feature = "debug_api"))]
#[ic_cdk::post_upgrade(decode_with = "decode_poke_upgrade")]
fn post_upgrade(callers: Option<Vec<Principal>>) {
    restore_poke_callers(callers);
    scheduler::start();
    aliases::start();
}

#[ic_cdk::query]
fn get_pricing() -> Pricing {
    pricing::get_pricing()
}

#[ic_cdk::query]
fn get_instance() -> InstanceInfo {
    instance::get_instance()
}

#[ic_cdk::query]
fn preview_subscription(suffix: String) -> Result<memo::SubscriptionPreview, String> {
    memo::preview(suffix)
}

fn decode_poke_upgrade(bytes: Vec<u8>) -> Option<Vec<Principal>> {
    if bytes.is_empty() {
        return None;
    }
    // Do not silently discard an incompatible argument through Candid's opt rule.
    let mut config = candid::DecoderConfig::new();
    config.set_skipping_quota(0);
    let mut decoder = candid::de::IDLDeserialize::new_with_config(&bytes, &config)
        .expect("invalid upgrade arguments");
    let callers = decoder
        .get_value()
        .expect("invalid poke allowlist argument");
    assert!(decoder.is_done(), "unexpected extra upgrade arguments");
    decoder.done().expect("invalid upgrade arguments");
    callers
}

fn configure_poke_callers(callers: Option<Vec<Principal>>, canonical: bool) {
    if let Some(callers) = callers {
        instance::validate_poke_callers(&callers, canonical)
            .unwrap_or_else(|error| ic_cdk::trap(error));
        state::set_poke_callers(callers);
    }
}

fn restore_poke_callers(callers: Option<Vec<Principal>>) {
    state::initialize_if_needed();
    if callers.is_some() {
        let runtime = config::runtime();
        configure_poke_callers(callers, runtime.observed_ledger == runtime.icp_ledger);
    }
}

enum PokeWake {
    Poll,
    Admission,
}

fn authorised_poke(caller: Principal) -> Option<PokeWake> {
    let runtime = config::runtime();
    if runtime.observed_ledger == runtime.icp_ledger {
        state::is_allowed_poke_caller(caller).then_some(PokeWake::Poll)
    } else {
        (state::read_instance_config().icp_admission_notifier == Some(caller))
            .then_some(PokeWake::Admission)
    }
}

#[ic_cdk::inspect_message]
fn inspect_message() {
    if ic_cdk::api::msg_method_name() != "poke"
        || authorised_poke(ic_cdk::api::msg_caller()).is_some()
    {
        ic_cdk::api::accept_message();
    }
}

#[ic_cdk::update]
fn poke() {
    match authorised_poke(ic_cdk::api::msg_caller()) {
        Some(PokeWake::Poll) => scheduler::request_external_poll(),
        Some(PokeWake::Admission) => scheduler::request_admission(),
        None => ic_cdk::trap("unauthorised poke"),
    }
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
        pub sns_root: Option<Principal>,
        pub icp_admission_notifier: Option<Principal>,
        pub allowed_poke_callers: Option<Vec<Principal>>,
    }

    impl From<&DebugInitArgs> for config::RuntimeConfig {
        fn from(value: &DebugInitArgs) -> Self {
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
        instance::validate_init(
            args.observed_ledger,
            args.sns_root,
            args.surplus_canister,
            args.icp_admission_notifier,
            args.icp_ledger,
            ic_cdk::api::canister_self(),
        );
        state::initialize_instance_config(instance::InstanceConfig {
            observed_ledger: args.observed_ledger,
            observed_profile: None,
            sns_root: args.sns_root,
            surplus_canister: args.surplus_canister,
            icp_admission_notifier: args.icp_admission_notifier,
        });
        state::write_debug_config((&args).into());
        configure_poke_callers(
            args.allowed_poke_callers,
            args.observed_ledger == args.icp_ledger,
        );
        // Debug builds are manually driven to keep PocketIC tests deterministic.
    }

    #[ic_cdk::post_upgrade(decode_with = "decode_poke_upgrade")]
    fn post_upgrade(callers: Option<Vec<Principal>>) {
        restore_poke_callers(callers);
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
    async fn debug_refresh_aliases() {
        aliases::refresh().await;
    }
    #[ic_cdk::query]
    fn debug_alias_metadata() -> aliases::AliasMetadata {
        state::alias_metadata()
    }
    #[ic_cdk::update]
    async fn debug_poll_once() {
        scheduler::debug_poll_once().await;
    }

    #[ic_cdk::update]
    fn debug_set_abort_after_staged_admission(enabled: bool) {
        polling::debug_set_abort_after_staged_admission(enabled);
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
    fn debug_set_scheduler_liquid_cycles_override(value: Option<u128>) {
        scheduler::debug_set_liquid_cycles_override(value);
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

    #[ic_cdk::update]
    fn debug_arm_scheduler_trap(lane: String) {
        scheduler::debug_arm_post_await_trap(&lane)
    }

    #[ic_cdk::query]
    fn debug_scheduler_lane_state(lane: String) -> Vec<bool> {
        let (running, scheduled) = scheduler::debug_lane_state(&lane);
        vec![running, scheduled]
    }

    #[ic_cdk::update]
    fn debug_arm_scheduler_overlap(lane: String) {
        scheduler::debug_arm_overlap(&lane)
    }

    #[ic_cdk::query]
    fn debug_scheduler_lane_counts(lane: String) -> Vec<u64> {
        let (starts, busy) = scheduler::debug_lane_counts(&lane);
        vec![starts, busy]
    }

    #[ic_cdk::update]
    async fn debug_scheduler_overlap(lane: String) {
        let (running_before, _) = scheduler::debug_lane_state(&lane);
        assert!(
            running_before,
            "overlap fixture requires an active owning worker for {lane}"
        );

        let (starts_before, busy_before) = scheduler::debug_lane_counts(&lane);
        for attempt in 1_u64..=2 {
            scheduler::debug_run_lane(&lane).await;

            let (running_after, _) = scheduler::debug_lane_state(&lane);
            let (starts_after, busy_after) = scheduler::debug_lane_counts(&lane);
            assert!(
                running_after,
                "busy {lane} attempt {attempt} released the owning worker's lane"
            );
            assert_eq!(
                starts_after, starts_before,
                "busy {lane} attempt {attempt} started another worker"
            );
            assert_eq!(
                busy_after,
                busy_before + attempt,
                "busy {lane} attempt {attempt} did not follow the rejection path"
            );
        }
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
        target: WatchTarget,
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugCursorArgs {
        admission_bootstrapped: bool,
        admission_next_block: u64,
        observed_bootstrapped: bool,
        observed_next_block: u64,
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugSeedSubscribers {
        kind: String,
        start: u64,
        count: u64,
    }

    #[ic_cdk::update]
    fn debug_seed_subscribers(args: DebugSeedSubscribers) {
        state::debug_seed_subscribers(&args.kind, args.start, args.count)
    }

    #[derive(CandidType, Deserialize)]
    pub struct DebugExternalSeed {
        subject: Principal,
        start: u64,
        count: u64,
    }
    #[ic_cdk::update]
    fn debug_seed_external(args: DebugExternalSeed) {
        assert!(args.count <= 128);
        for index in args.start..args.start + args.count {
            let mut bytes = vec![0x7f];
            bytes.extend_from_slice(&index.to_be_bytes());
            bytes.push(1);
            state::put_relation(subscription::SubjectSubscription {
                subject: args.subject,
                subscription: subscription::Subscription {
                    subscriber: Principal::from_slice(&bytes),
                    target: WatchTarget::Subaccount(7),
                    minimum_units: candid::Nat::from(index % 16),
                },
            });
        }
    }

    #[ic_cdk::query]
    fn debug_poke_counts() -> Vec<u64> {
        let (attempts, accepted) = clients::subscriber::debug_poke_counts();
        vec![attempts, accepted]
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
        state::get_target_subscription(args.subscriber, args.target)
    }

    #[ic_cdk::query]
    fn debug_global_subscription(subscriber: Principal) -> bool {
        state::has_global_subscriber(subscriber)
    }
}

ic_cdk::export_candid!();

#[cfg(test)]
mod poke_tests {
    use super::*;

    #[test]
    fn upgrade_argument_distinguishes_preserve_replace_and_clear() {
        assert_eq!(decode_poke_upgrade(vec![]), None);
        assert_eq!(decode_poke_upgrade(candid::encode_args(()).unwrap()), None);
        assert_eq!(decode_poke_upgrade(candid::encode_one(()).unwrap()), None);
        let worker = Principal::from_slice(&[1, 2]);
        for policy in [Some(vec![worker]), Some(vec![]), None] {
            assert_eq!(
                decode_poke_upgrade(candid::encode_one(&policy).unwrap()),
                policy
            );
        }
        for invalid in [
            b"invalid".to_vec(),
            candid::encode_one("invalid").unwrap(),
            candid::encode_one(Some(vec!["invalid"])).unwrap(),
            candid::encode_args((None::<Vec<Principal>>, 1u8)).unwrap(),
        ] {
            assert!(std::panic::catch_unwind(|| decode_poke_upgrade(invalid)).is_err());
        }
    }
}
