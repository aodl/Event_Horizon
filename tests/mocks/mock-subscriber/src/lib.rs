#![allow(dead_code)]
use candid::{CandidType, Nat, Principal};
use serde::Deserialize;
use std::cell::RefCell;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
enum PokeTarget {
    #[serde(rename = "subaccount")]
    Subaccount(u64),
    #[serde(rename = "neuron_nonce")]
    NeuronNonce(u64),
}
#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct PokeMatch {
    target: PokeTarget,
    first_block_index: u64,
    max_amount: Nat,
}

#[derive(Default)]
struct State {
    pokes: u64,
    last_matches: Vec<PokeMatch>,
    last_caller: Option<Principal>,
    trap: bool,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[ic_cdk::init]
fn init() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

#[ic_cdk::update]
fn poke(matches: Vec<PokeMatch>) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pokes += 1;
        s.last_matches = matches;
        s.last_caller = Some(ic_cdk::api::msg_caller());
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
fn debug_last_matches() -> Vec<PokeMatch> {
    STATE.with(|s| s.borrow().last_matches.clone())
}

#[ic_cdk::query]
fn debug_last_caller() -> Option<Principal> {
    STATE.with(|s| s.borrow().last_caller)
}

#[ic_cdk::update]
fn debug_set_trap(value: bool) {
    STATE.with(|s| s.borrow_mut().trap = value)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn generated_poke_record_matches_normative_candid() {
        let generated = super::__export_service();
        let normative = include_str!("../../../../candid/subscriber.did");
        fn fields(did: &str) -> BTreeSet<String> {
            did.split("type PokeMatch = record {")
                .nth(1)
                .unwrap()
                .split('}')
                .next()
                .unwrap()
                .split(';')
                .map(|s| s.split_whitespace().collect::<String>())
                .filter(|s| !s.is_empty())
                .collect()
        }
        assert_eq!(fields(&generated), fields(normative));
        assert_eq!(fields(&generated).len(), 3);
        assert!(generated.contains("poke : (vec PokeMatch) -> ()"));
    }
}
