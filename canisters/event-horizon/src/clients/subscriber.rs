use candid::{CandidType, Nat, Principal};
use ic_cdk::call::Call;
use serde::Deserialize;

use crate::subscription::WatchTarget;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct PokeMatch {
    pub target: WatchTarget,
    pub max_amount: Nat,
}

use crate::config::RESERVE_PROTECTION_CYCLES;

pub fn poke(subscriber: Principal, matches: Vec<PokeMatch>) -> bool {
    let call = Call::bounded_wait(subscriber, "poke").with_arg(&matches);
    let required = call.get_cost();
    let balance = ic_cdk::api::canister_liquid_cycle_balance();
    if balance < RESERVE_PROTECTION_CYCLES.saturating_add(required) {
        return false;
    }
    call.oneway().is_ok()
}
