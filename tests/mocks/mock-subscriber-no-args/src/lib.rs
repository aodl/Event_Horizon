use candid::Principal;
use std::cell::RefCell;

#[derive(Default)]
struct State {
    pokes: u64,
    last_caller: Option<Principal>,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[ic_cdk::init]
fn init() {
    STATE.with(|state| *state.borrow_mut() = State::default());
}

#[ic_cdk::update]
fn poke() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.pokes += 1;
        state.last_caller = Some(ic_cdk::api::msg_caller());
    });
}

#[ic_cdk::query]
fn debug_pokes() -> u64 {
    STATE.with(|state| state.borrow().pokes)
}

#[ic_cdk::query]
fn debug_last_caller() -> Option<Principal> {
    STATE.with(|state| state.borrow().last_caller)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    #[test]
    fn generated_candid_has_zero_argument_update_poke() {
        let generated = super::__export_service();
        let poke_lines: Vec<_> = generated
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("poke :"))
            .collect();

        assert_eq!(poke_lines, ["poke : () -> ();"]);
        assert!(generated.contains("debug_pokes : () -> (nat64) query;"));
        assert!(generated.contains("debug_last_caller : () -> (opt principal) query;"));
    }
}
