use candid::{CandidType, Principal};
use serde::Deserialize;
use std::cell::RefCell;

#[derive(Clone, CandidType, Deserialize, Default)]
struct SnsCanisters {
    root: Option<Principal>,
    ledger: Option<Principal>,
    governance: Option<Principal>,
}
#[derive(CandidType, Deserialize)]
struct Empty {}

thread_local! { static VALUE: RefCell<SnsCanisters> = RefCell::new(SnsCanisters::default()); }

#[ic_cdk::init]
fn init(value: SnsCanisters) {
    VALUE.with(|slot| *slot.borrow_mut() = value);
}

#[ic_cdk::query]
fn list_sns_canisters(_: Empty) -> SnsCanisters {
    VALUE.with(|slot| slot.borrow().clone())
}

ic_cdk::export_candid!();
