use candid::Principal;
use std::cell::RefCell;

#[derive(Default)]
struct State {
    trusted_event_horizon: Option<Principal>,
    pokes: u64,
    last_caller: Option<Principal>,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[ic_cdk::init]
fn init(trusted_event_horizon: Principal) {
    STATE.with_borrow_mut(|state| {
        *state = State {
            trusted_event_horizon: Some(trusted_event_horizon),
            ..State::default()
        };
    });
}

// A genuine zero-argument entry point using the pinned CDK's ordinary macro.
// It deliberately cannot inspect the notification's global flag or matches.
#[ic_cdk::update]
fn poke() {
    let caller = ic_cdk::api::msg_caller();
    STATE.with_borrow_mut(|state| {
        if state.trusted_event_horizon != Some(caller) {
            ic_cdk::trap("untrusted Event Horizon caller");
        }
        state.pokes += 1;
        state.last_caller = Some(caller);
    });
}

#[ic_cdk::query]
fn debug_pokes() -> u64 {
    STATE.with_borrow(|state| state.pokes)
}

#[ic_cdk::query]
fn debug_last_caller() -> Option<Principal> {
    STATE.with_borrow(|state| state.last_caller)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    #[test]
    fn exported_index_poke_genuinely_takes_zero_arguments() {
        let service = super::__export_service();
        assert!(service.contains("poke : () -> ()"), "{service}");
        assert!(service.contains("debug_pokes : () -> (nat64) query"));
        assert!(service.contains("debug_last_caller : () -> (opt principal) query"));
        assert!(!service.contains("PokeNotification"));
    }
}
