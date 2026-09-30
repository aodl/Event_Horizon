use candid::{CandidType, Nat, Principal};
use ic_cdk::call::Call;
use serde::Deserialize;

#[cfg(feature = "debug_api")]
use std::cell::Cell;

use crate::subscription::WatchTarget;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct PokeMatch {
    pub target: WatchTarget,
    pub max_amount: Nat,
}

use crate::config::RESERVE_PROTECTION_CYCLES;

#[cfg(feature = "debug_api")]
thread_local! {
    static DEBUG_POKE_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
    static DEBUG_POKE_ACCEPTED: Cell<u64> = const { Cell::new(0) };
}

pub fn poke(subscriber: Principal, matches: Vec<PokeMatch>) -> bool {
    #[cfg(feature = "debug_api")]
    DEBUG_POKE_ATTEMPTS.with(|value| value.set(value.get().saturating_add(1)));
    let call = Call::bounded_wait(subscriber, "poke").with_arg(&matches);
    let required = call.get_cost();
    let balance = ic_cdk::api::canister_liquid_cycle_balance();
    if balance < RESERVE_PROTECTION_CYCLES.saturating_add(required) {
        return false;
    }
    let accepted = call.oneway().is_ok();
    #[cfg(feature = "debug_api")]
    if accepted {
        DEBUG_POKE_ACCEPTED.with(|value| value.set(value.get().saturating_add(1)));
    }
    accepted
}

#[cfg(feature = "debug_api")]
pub fn debug_poke_counts() -> (u64, u64) {
    (
        DEBUG_POKE_ATTEMPTS.with(Cell::get),
        DEBUG_POKE_ACCEPTED.with(Cell::get),
    )
}
