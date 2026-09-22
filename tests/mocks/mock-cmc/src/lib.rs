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
}
impl Default for State {
    fn default() -> Self {
        Self {
            behavior: Behavior::Ok,
            calls: 0,
        }
    }
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

ic_cdk::export_candid!();
