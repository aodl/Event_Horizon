use candid::{CandidType, Nat, Principal};
use ic_cdk::call::Call;
use serde::Deserialize;

#[cfg(feature = "debug_api")]
use std::cell::Cell;

use crate::{config::RESERVE_PROTECTION_CYCLES, subscription::WatchTarget};

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct PokeMatch {
    pub subject: Principal,
    pub target: WatchTarget,
    pub first_block_index: u64,
    pub max_amount: Nat,
}

#[derive(Default, Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct PokeNotification {
    pub global_activity: bool,
    pub matches: Vec<PokeMatch>,
}

#[cfg(feature = "debug_api")]
thread_local! {
    static DEBUG_POKE_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
    static DEBUG_POKE_ACCEPTED: Cell<u64> = const { Cell::new(0) };
}

#[cfg(test)]
thread_local! { static TEST_GLOBAL_ENCODINGS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

/// A dispatch batch owns this optional constant-size buffer. Every recipient still
/// creates its own ordinary bounded-wait oneway Call and checks its actual cost.
fn global_arguments<'a>(
    cache: &'a mut Option<Vec<u8>>,
    notification: &PokeNotification,
) -> Option<&'a [u8]> {
    (notification.global_activity && notification.matches.is_empty()).then(|| {
        cache
            .get_or_insert_with(|| {
                #[cfg(test)]
                TEST_GLOBAL_ENCODINGS.with(|n| n.set(n.get() + 1));
                candid::encode_one(notification).expect("encode global notification")
            })
            .as_slice()
    })
}

pub fn poke(
    subscriber: Principal,
    notification: PokeNotification,
    global_cache: &mut Option<Vec<u8>>,
) -> bool {
    if !notification.global_activity && notification.matches.is_empty() {
        return false;
    }
    #[cfg(feature = "debug_api")]
    DEBUG_POKE_ATTEMPTS.with(|v| v.set(v.get().saturating_add(1)));
    let call = if let Some(bytes) = global_arguments(global_cache, &notification) {
        Call::bounded_wait(subscriber, "poke").with_raw_args(bytes)
    } else {
        Call::bounded_wait(subscriber, "poke").with_arg(&notification)
    };
    if ic_cdk::api::canister_liquid_cycle_balance()
        < RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
    {
        return false;
    }
    let accepted = call.oneway().is_ok();
    #[cfg(feature = "debug_api")]
    if accepted {
        DEBUG_POKE_ACCEPTED.with(|v| v.set(v.get().saturating_add(1)));
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

#[cfg(test)]
mod encoding_tests {
    use super::*;
    #[test]
    fn global_batch_reuses_exact_one_record_encoding_without_affecting_specifics() {
        let notification = PokeNotification {
            global_activity: true,
            matches: vec![],
        };
        let typed = candid::encode_one(&notification).unwrap();
        let mut cache = None;
        let first = global_arguments(&mut cache, &notification).unwrap();
        assert_eq!(first, typed);
        let pointer = first.as_ptr();
        for _ in 0..1024 {
            let raw = global_arguments(&mut cache, &notification).unwrap();
            assert_eq!(raw.as_ptr(), pointer);
            assert_eq!(
                candid::decode_one::<PokeNotification>(raw).unwrap(),
                notification
            );
        }
        let mut specific = notification.clone();
        specific.matches.push(PokeMatch {
            subject: Principal::from_slice(&[1, 1]),
            target: WatchTarget::Subaccount(u64::MAX),
            first_block_index: u64::MAX,
            max_amount: Nat(Nat::from(1u8).0 << 32768usize),
        });
        assert!(global_arguments(&mut cache, &specific).is_none());
        specific.global_activity = false;
        assert!(global_arguments(&mut cache, &specific).is_none());
        assert!(global_arguments(&mut cache, &PokeNotification::default()).is_none());
        assert_eq!(cache.unwrap(), typed);
        assert_eq!(TEST_GLOBAL_ENCODINGS.with(std::cell::Cell::get), 1);
    }
}
