use candid::{CandidType, Principal};
use serde::Deserialize;
use std::cell::{Cell, RefCell};

#[derive(Clone, CandidType, Deserialize, Default)]
struct SnsCanisters {
    root: Option<Principal>,
    ledger: Option<Principal>,
    governance: Option<Principal>,
}
#[derive(CandidType, Deserialize)]
struct Empty {}

thread_local! {
    static VALUE: RefCell<SnsCanisters> = RefCell::new(SnsCanisters::default());
    static UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
}

#[ic_cdk::init]
fn init(value: SnsCanisters) {
    VALUE.with(|slot| *slot.borrow_mut() = value);
}

#[ic_cdk::query]
fn list_sns_canisters(_: Empty) -> SnsCanisters {
    if UNAVAILABLE.with(Cell::get) {
        ic_cdk::trap("mock SNS Root unavailable");
    }
    VALUE.with(|slot| slot.borrow().clone())
}

#[ic_cdk::update]
fn debug_set_canisters(value: SnsCanisters) {
    VALUE.with(|slot| *slot.borrow_mut() = value);
}

#[ic_cdk::update]
fn debug_set_unavailable(value: bool) {
    UNAVAILABLE.with(|slot| slot.set(value));
}

ic_cdk::export_candid!();
