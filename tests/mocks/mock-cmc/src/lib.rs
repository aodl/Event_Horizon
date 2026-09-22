#![allow(dead_code)]
use candid::{CandidType, Deserialize, Nat, Principal};
use std::cell::RefCell;
#[derive(Clone, Debug, CandidType, Deserialize)]
struct NotifyTopUpArg {
    canister_id: Principal,
    block_index: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
enum NotifyError {
    Refunded {
        reason: String,
        block_index: Option<u64>,
    },
    Processing,
    TransactionTooOld(u64),
    InvalidTransaction(String),
    Other {
        error_code: u64,
        error_message: String,
    },
}
type NotifyResult = Result<Nat, NotifyError>;
#[derive(Clone, Debug, CandidType, Deserialize)]
enum Behavior {
    Ok,
    Refunded,
    Processing,
    TransactionTooOld,
    InvalidTransaction,
    Other,
}
struct State {
    behavior: Behavior,
    calls: u64,
    pricing_calls: u64,
    xdr_permyriad_per_icp: u64,
    rate_timestamp_seconds: u64,
    pricing_fail: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            behavior: Behavior::Ok,
            calls: 0,
            pricing_calls: 0,
            xdr_permyriad_per_icp: 10_000,
            rate_timestamp_seconds: 0,
            pricing_fail: false,
        }
    }
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct IcpXdrConversionRate {
    timestamp_seconds: u64,
    xdr_permyriad_per_icp: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct IcpXdrConversionRateResponse {
    data: IcpXdrConversionRate,
    hash_tree: Vec<u8>,
    certificate: Vec<u8>,
}

#[ic_cdk::query]
fn get_icp_xdr_conversion_rate() -> IcpXdrConversionRateResponse {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pricing_calls += 1;
        if s.pricing_fail {
            ic_cdk::trap("forced pricing failure");
        }
        IcpXdrConversionRateResponse {
            data: IcpXdrConversionRate {
                timestamp_seconds: if s.rate_timestamp_seconds == 0 {
                    ic_cdk::api::time() / 1_000_000_000
                } else {
                    s.rate_timestamp_seconds
                },
                xdr_permyriad_per_icp: s.xdr_permyriad_per_icp,
            },
            hash_tree: vec![],
            certificate: vec![],
        }
    })
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}
#[ic_cdk::init]
fn init() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}
#[ic_cdk::update]
fn notify_top_up(_arg: NotifyTopUpArg) -> NotifyResult {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.calls += 1;
        match s.behavior {
            Behavior::Ok => Ok(Nat::from(1_000_000_000_000u64)),
            Behavior::Refunded => Err(NotifyError::Refunded {
                reason: "forced".into(),
                block_index: Some(0),
            }),
            Behavior::Processing => Err(NotifyError::Processing),
            Behavior::TransactionTooOld => Err(NotifyError::TransactionTooOld(0)),
            Behavior::InvalidTransaction => Err(NotifyError::InvalidTransaction("forced".into())),
            Behavior::Other => Err(NotifyError::Other {
                error_code: 1,
                error_message: "forced".into(),
            }),
        }
    })
}
#[ic_cdk::update]
fn debug_set_behavior(value: Behavior) {
    STATE.with(|s| s.borrow_mut().behavior = value)
}
#[ic_cdk::query]
fn debug_calls() -> u64 {
    STATE.with(|s| s.borrow().calls)
}

#[ic_cdk::update]
fn debug_set_rate(value: u64) {
    STATE.with(|s| s.borrow_mut().xdr_permyriad_per_icp = value)
}

#[ic_cdk::update]
fn debug_set_rate_timestamp(value: u64) {
    STATE.with(|s| s.borrow_mut().rate_timestamp_seconds = value)
}

#[ic_cdk::update]
fn debug_set_pricing_fail(value: bool) {
    STATE.with(|s| s.borrow_mut().pricing_fail = value)
}

#[ic_cdk::query]
fn debug_pricing_calls() -> u64 {
    STATE.with(|s| s.borrow().pricing_calls)
}

ic_cdk::export_candid!();
