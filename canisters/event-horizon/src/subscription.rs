use candid::{CandidType, Nat, Principal};
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

use crate::{account::numbered_subaccount, memo::SubscriptionDeclaration};

/// `minimum_units == 0` means the declaration omitted a threshold and every
/// incoming transfer to this watched account is relevant.
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub subscriber: Principal,
    pub numbered_subaccount: u8,
    pub minimum_units: Nat,
}

impl Subscription {
    pub fn from_declaration(
        value: SubscriptionDeclaration,
        decimals: u8,
    ) -> Result<Self, crate::memo::MemoParseError> {
        match value {
            SubscriptionDeclaration::Account {
                subscriber,
                numbered_subaccount,
                minimum,
            } => Ok(Self {
                subscriber,
                numbered_subaccount,
                minimum_units: minimum
                    .map(|value| value.to_units(decimals))
                    .transpose()?
                    .unwrap_or_else(|| Nat::from(0u8)),
            }),
            SubscriptionDeclaration::Global { .. } | SubscriptionDeclaration::Range { .. } => {
                panic!("non-account declaration cannot become one account subscription")
            }
        }
    }
    pub fn account(&self) -> Account {
        Account {
            owner: self.subscriber,
            subaccount: Some(numbered_subaccount(self.numbered_subaccount)),
        }
    }

    pub fn matches(&self, amount_units: &Nat) -> bool {
        self.minimum_units == Nat::from(0u8) || amount_units >= &self.minimum_units
    }
}

/// Same account + lower threshold subsumes higher threshold. Because declarations
/// are permanent, an effective threshold can only stay unchanged or become less
/// restrictive. The internal zero sentinel is the lowest possible threshold and
/// therefore naturally represents an unfiltered "all incoming transfers"
/// subscription.
pub fn merge_subscription(existing: Option<Subscription>, candidate: Subscription) -> Subscription {
    match existing {
        Some(existing)
            if existing.subscriber == candidate.subscriber
                && existing.numbered_subaccount == candidate.numbered_subaccount
                && existing.minimum_units <= candidate.minimum_units =>
        {
            existing
        }
        _ => candidate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Principal {
        Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap()
    }

    #[test]
    fn lower_threshold_replaces_higher_for_same_account() {
        let high = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(100_000_000u64),
        };
        let low = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(1_000_000u64),
        };
        assert_eq!(merge_subscription(Some(high), low.clone()), low);
    }

    #[test]
    fn unfiltered_subscription_subsumes_thresholded_subscription() {
        let thresholded = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(1_000_000u64),
        };
        let unfiltered = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(0u8),
        };
        assert_eq!(
            merge_subscription(Some(thresholded), unfiltered.clone()),
            unfiltered
        );
    }

    #[test]
    fn higher_threshold_does_not_reduce_service() {
        let low = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(1_000_000u64),
        };
        let high = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(100_000_000u64),
        };
        assert_eq!(merge_subscription(Some(low.clone()), high), low);
    }

    #[test]
    fn threshold_is_inclusive() {
        let sub = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(1_000_000u64),
        };
        assert!(!sub.matches(&Nat::from(999_999u64)));
        assert!(sub.matches(&Nat::from(1_000_000u64)));
        assert!(sub.matches(&Nat::from(1_000_001u64)));
    }

    #[test]
    fn omitted_threshold_matches_every_transfer_amount() {
        let sub = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_units: Nat::from(0u8),
        };
        assert!(sub.matches(&Nat::from(0u8)));
        assert!(sub.matches(&Nat::from(1u8)));
        assert!(sub.matches(&Nat::from(u128::MAX)));
    }
}
