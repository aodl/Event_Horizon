#![allow(dead_code)]
use candid::{CandidType, Nat, Principal};
use serde::Deserialize;
use sha2::{Digest, Sha256};
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
    subject: Principal,
    target: PokeTarget,
    first_block_index: u64,
    max_amount: Nat,
}

#[derive(Clone, Debug, Default, CandidType, Deserialize, PartialEq, Eq)]
struct PokeNotification {
    global_activity: bool,
    matches: Vec<PokeMatch>,
}

#[derive(Default)]
struct State {
    pokes: u64,
    last_notification: PokeNotification,
    last_caller: Option<Principal>,
    trap: bool,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[ic_cdk::init]
fn init() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

#[ic_cdk::update]
fn poke(notification: PokeNotification) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pokes += 1;
        s.last_notification = notification;
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
    STATE.with(|s| s.borrow().last_notification.matches.clone())
}

#[ic_cdk::query]
fn debug_last_notification() -> PokeNotification {
    STATE.with(|s| s.borrow().last_notification.clone())
}

// An update has the ordinary 40B instruction budget. Returning the full large
// stress fixture from a query would spend its lower 5B budget serializing it.
#[ic_cdk::update]
fn debug_notification_digest() -> Vec<u8> {
    STATE.with(|s| {
        Sha256::digest(candid::encode_one(&s.borrow().last_notification).unwrap()).to_vec()
    })
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
        fn fields(did: &str, record: &str) -> BTreeSet<String> {
            did.split(&format!("type {record} = record {{"))
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
        for record in ["PokeMatch", "PokeNotification"] {
            assert_eq!(fields(&generated, record), fields(normative, record));
        }
        assert_eq!(fields(&generated, "PokeMatch").len(), 4);
        assert!(generated.contains("poke : (PokeNotification) -> ()"));
    }
}
