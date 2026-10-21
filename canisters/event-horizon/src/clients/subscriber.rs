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

pub fn poke(subscriber: Principal, matches: Vec<PokeMatch>) -> bool {
    #[cfg(feature = "debug_api")]
    DEBUG_POKE_ATTEMPTS.with(|value| value.set(value.get().saturating_add(1)));
    // This remains one payload-bearing wire format. Candid receivers that declare
    // `poke : () -> ()` ignore this extra argument through argument subtyping.
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
