#![allow(dead_code)]
use std::cell::RefCell;

#[derive(Default)]
struct State {
    pokes: u64,
    last_subaccounts: Vec<u8>,
    trap: bool,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[ic_cdk::init]
fn init() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

#[ic_cdk::update]
fn poke(subaccounts: Vec<u8>) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pokes += 1;
        s.last_subaccounts = subaccounts;
        if s.trap {
            ic_cdk::trap("forced subscriber trap")
        }
    })
}

#[ic_cdk::query]
fn debug_pokes() -> u64 {
    STATE.with(|s| s.borrow().pokes)
}

#[ic_cdk::query]
fn debug_last_subaccounts() -> Vec<u8> {
    STATE.with(|s| s.borrow().last_subaccounts.clone())
}

#[ic_cdk::update]
fn debug_set_trap(value: bool) {
    STATE.with(|s| s.borrow_mut().trap = value)
}

ic_cdk::export_candid!();
