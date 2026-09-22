use candid::Principal;
use ic_cdk::call::Call;

use crate::config::RESERVE_PROTECTION_CYCLES;

pub fn poke(subscriber: Principal, subaccounts: Vec<u8>) -> bool {
    let call = Call::bounded_wait(subscriber, "poke").with_arg(&subaccounts);
    let required = call.get_cost();
    let balance = ic_cdk::api::canister_liquid_cycle_balance();
    if balance < RESERVE_PROTECTION_CYCLES.saturating_add(required) {
        return false;
    }
    call.oneway().is_ok()
}
