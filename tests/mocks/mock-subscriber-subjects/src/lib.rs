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

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct SubjectPokeMatch {
    subject: Principal,
    target: PokeTarget,
    first_block_index: u64,
    max_amount: Nat,
}
#[derive(Default)]
struct State {
    pokes: u64,
    subject_pokes: u64,
    last_subjects: Vec<SubjectPokeMatch>,
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
fn poke(matches: Vec<PokeMatch>, subjects: Option<Vec<SubjectPokeMatch>>) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pokes += 1;
        s.last_matches = matches;
        s.last_subjects = subjects.unwrap_or_default();
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

#[ic_cdk::update]
fn poke_subjects(matches: Vec<SubjectPokeMatch>) {
    STATE.with_borrow_mut(|s| {
        if s.trap {
            ic_cdk::trap("forced subject trap")
        }
        s.subject_pokes += 1;
        s.last_subjects = matches;
        s.last_matches.clear();
        s.last_caller = Some(ic_cdk::api::msg_caller());
    });
}
#[ic_cdk::query]
fn debug_subject_pokes() -> u64 {
    STATE.with_borrow(|s| s.subject_pokes)
}
#[ic_cdk::query]
fn debug_last_subjects() -> Vec<SubjectPokeMatch> {
    STATE.with_borrow(|s| s.last_subjects.clone())
}

#[cfg(test)]
mod contract_tests {
    #[test]
    fn generated_subject_callback_matches_normative_contract() {
        let generated = super::__export_service();
        let expected = include_str!("../../../../candid/subscriber.did");
        for declaration in [
            "type PokeMatch = record {",
            "type SubjectPokeMatch = record {",
        ] {
            let fields = |s: &str| {
                s.split(declaration)
                    .nth(1)
                    .unwrap()
                    .split('}')
                    .next()
                    .unwrap()
                    .split(';')
                    .map(|v| v.split_whitespace().collect::<String>())
                    .filter(|s| !s.is_empty())
                    .collect::<std::collections::BTreeSet<_>>()
            };
            assert_eq!(fields(&generated), fields(expected));
        }
        assert!(generated.contains("poke : (vec PokeMatch, opt vec SubjectPokeMatch) -> ()"));
        assert!(generated.contains("poke_subjects : (vec SubjectPokeMatch) -> ()"));
    }
}

ic_cdk::export_candid!();
