use candid::{CandidType, Nat, Principal};
use ic_cdk::call::Call;
use serde::Deserialize;

#[cfg(feature = "debug_api")]
use std::cell::Cell;

use crate::subscription::WatchTarget;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct PokeMatch {
    pub target: WatchTarget,
    pub first_block_index: u64,
    pub max_amount: Nat,
}

use crate::config::RESERVE_PROTECTION_CYCLES;

#[cfg(feature = "debug_api")]
thread_local! {
    static DEBUG_POKE_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
    static DEBUG_POKE_ACCEPTED: Cell<u64> = const { Cell::new(0) };
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct SubjectPokeMatch {
    pub subject: Principal,
    pub target: WatchTarget,
    pub first_block_index: u64,
    pub max_amount: Nat,
}
#[derive(Default, Debug)]
pub struct Notification {
    pub global: bool,
    pub self_matches: Vec<PokeMatch>,
    pub subject_matches: Vec<SubjectPokeMatch>,
}
impl Notification {
    pub fn method(&self) -> &'static str {
        if !self.global && self.self_matches.is_empty() && !self.subject_matches.is_empty() {
            "poke_subjects"
        } else {
            "poke"
        }
    }
}
pub fn poke(subscriber: Principal, notification: Notification) -> bool {
    #[cfg(feature = "debug_api")]
    DEBUG_POKE_ATTEMPTS.with(|v| v.set(v.get().saturating_add(1)));
    let call = Call::bounded_wait(subscriber, notification.method());
    let call = if notification.method() == "poke_subjects" {
        call.with_arg(&notification.subject_matches)
    } else {
        let external =
            (!notification.subject_matches.is_empty()).then_some(notification.subject_matches);
        call.with_args(&(notification.self_matches, external))
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use candid::{decode_args, encode_args, encode_one, Nat};

    use super::PokeMatch;
    use crate::subscription::WatchTarget;

    fn representative_matches() -> Vec<PokeMatch> {
        vec![
            PokeMatch {
                target: WatchTarget::Subaccount(7),
                first_block_index: (1u64 << 53) + 1,
                max_amount: Nat::from(u64::MAX),
            },
            PokeMatch {
                target: WatchTarget::NeuronNonce(u64::MAX),
                first_block_index: u64::MAX,
                max_amount: Nat::from_str("18446744073709551616").expect("u64 max plus one"),
            },
        ]
    }

    fn assert_zero_argument_receiver_accepts(bytes: &[u8]) {
        decode_args::<()>(bytes).expect("zero-argument receiver accepts extra arguments");
    }

    #[test]
    fn two_argument_sequence_preserves_subjects_and_legacy_receivers() {
        use super::{Notification, SubjectPokeMatch};
        use candid::{CandidType, Deserialize, Principal};
        #[derive(CandidType, Deserialize)]
        struct OldMatch {
            target: WatchTarget,
            max_amount: Nat,
        }
        let own = representative_matches();
        let external: Vec<_> = (0..256)
            .map(|n| SubjectPokeMatch {
                subject: Principal::from_slice(&[n as u8, 1]),
                target: WatchTarget::Subaccount(n),
                first_block_index: u64::MAX - n,
                max_amount: Nat::from_str("340282366920938463463374607431768211456").unwrap(),
            })
            .collect();
        let bytes = encode_args((&own, Some(&external))).unwrap();
        let (a, b): (Vec<PokeMatch>, Option<Vec<SubjectPokeMatch>>) = decode_args(&bytes).unwrap();
        assert_eq!(a, own);
        assert_eq!(b, Some(external.clone()));
        let (old,): (Vec<OldMatch>,) = decode_args(&bytes).unwrap();
        assert_eq!(old.len(), own.len());
        assert_eq!(old[0].target, own[0].target);
        assert_eq!(old[0].max_amount, own[0].max_amount);
        decode_args::<()>(&bytes).unwrap();
        let normal = encode_args((&own, Option::<Vec<SubjectPokeMatch>>::None)).unwrap();
        println!(
            "callback_shape_bytes self_old={} self_two_args={} global_old={} global_two_args={}",
            encode_one(&own).unwrap().len(),
            normal.len(),
            encode_one(Vec::<PokeMatch>::new()).unwrap().len(),
            encode_args((
                Vec::<PokeMatch>::new(),
                Option::<Vec<SubjectPokeMatch>>::None
            ))
            .unwrap()
            .len()
        );
        assert_eq!(decode_args::<(Vec<PokeMatch>,)>(&normal).unwrap().0, own);
        let mut notification = Notification {
            subject_matches: external,
            ..Default::default()
        };
        assert_eq!(notification.method(), "poke_subjects");
        notification.global = true;
        assert_eq!(notification.method(), "poke");
        notification.global = false;
        notification.self_matches = own;
        assert_eq!(notification.method(), "poke");
        println!(
            "callback_bytes self_only={} full_external_256={} mixed_encoding_fixture={}",
            normal.len(),
            encode_one(&notification.subject_matches).unwrap().len(),
            bytes.len()
        );
    }

    #[test]
    fn candid_poke_argument_compatibility_matrix() {
        // `encode_args(())` encodes an empty argument sequence. `encode_one(())`
        // instead encodes one Candid null value; the shared Rust `()` spelling
        // must not obscure that arity distinction.
        let zero_arguments = encode_args(()).expect("encode zero arguments");
        let one_null = encode_one(()).expect("encode one null argument");
        let empty_matches = encode_one(Vec::<PokeMatch>::new()).expect("encode empty vector");
        let matches = representative_matches();
        let nonempty_matches = encode_one(&matches).expect("encode matches");

        assert_ne!(zero_arguments, one_null);
        assert_ne!(zero_arguments, empty_matches);
        assert_ne!(one_null, empty_matches);

        assert_zero_argument_receiver_accepts(&zero_arguments);
        assert_zero_argument_receiver_accepts(&one_null);
        assert_zero_argument_receiver_accepts(&empty_matches);
        assert_zero_argument_receiver_accepts(&nonempty_matches);

        assert!(decode_args::<(Vec<PokeMatch>,)>(&zero_arguments).is_err());
        assert!(decode_args::<(Vec<PokeMatch>,)>(&one_null).is_err());
        assert_eq!(
            decode_args::<(Vec<PokeMatch>,)>(&empty_matches)
                .expect("vector receiver accepts empty vector")
                .0,
            Vec::new()
        );
        assert_eq!(
            decode_args::<(Vec<PokeMatch>,)>(&nonempty_matches)
                .expect("vector receiver accepts matches")
                .0,
            matches
        );
    }

    #[test]
    fn zero_argument_receiver_accepts_maximum_callback_payload() {
        let matches: Vec<_> = (0..256)
            .map(|value| PokeMatch {
                target: if value % 2 == 0 {
                    WatchTarget::Subaccount(value)
                } else {
                    WatchTarget::NeuronNonce(value)
                },
                first_block_index: u64::MAX - value,
                max_amount: Nat::from(value),
            })
            .collect();
        let bytes = encode_one(&matches).expect("encode maximum callback payload");

        assert_zero_argument_receiver_accepts(&bytes);
        assert_eq!(
            decode_args::<(Vec<PokeMatch>,)>(&bytes)
                .expect("vector receiver accepts maximum callback payload")
                .0,
            matches
        );
    }
}

#[cfg(feature = "debug_api")]
pub fn debug_poke_counts() -> (u64, u64) {
    (
        DEBUG_POKE_ATTEMPTS.with(Cell::get),
        DEBUG_POKE_ACCEPTED.with(Cell::get),
    )
}
