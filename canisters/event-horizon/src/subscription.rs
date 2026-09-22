use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::{
    account::{account_identifier_bytes, numbered_subaccount},
    memo::SubscriptionDeclaration,
};

/// `minimum_e8s == 0` means the declaration omitted a threshold and every
/// incoming transfer to this watched account is relevant.
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub subscriber: Principal,
    pub numbered_subaccount: u8,
    pub minimum_e8s: u64,
}

impl From<SubscriptionDeclaration> for Subscription {
    fn from(value: SubscriptionDeclaration) -> Self {
        Self {
            subscriber: value.subscriber,
            numbered_subaccount: value.numbered_subaccount,
            minimum_e8s: value.minimum_e8s,
        }
    }
}

impl Subscription {
    pub fn account_identifier(&self) -> [u8; 32] {
        account_identifier_bytes(
            self.subscriber,
            numbered_subaccount(self.numbered_subaccount),
        )
    }

    pub fn matches(&self, transfer_amount_e8s: u64) -> bool {
        self.minimum_e8s == 0 || transfer_amount_e8s >= self.minimum_e8s
    }
}

/// Same account + lower threshold subsumes higher threshold. The internal zero
/// sentinel is the lowest possible threshold and therefore naturally represents
/// an unfiltered "all incoming transfers" subscription.
pub fn merge_subscription(existing: Option<Subscription>, candidate: Subscription) -> Subscription {
    match existing {
        Some(existing)
            if existing.subscriber == candidate.subscriber
                && existing.numbered_subaccount == candidate.numbered_subaccount
                && existing.minimum_e8s <= candidate.minimum_e8s =>
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
            minimum_e8s: 100_000_000,
        };
        let low = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_e8s: 1_000_000,
        };
        assert_eq!(merge_subscription(Some(high), low.clone()), low);
    }

    #[test]
    fn unfiltered_subscription_subsumes_thresholded_subscription() {
        let thresholded = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_e8s: 1_000_000,
        };
        let unfiltered = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_e8s: 0,
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
            minimum_e8s: 1_000_000,
        };
        let high = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_e8s: 100_000_000,
        };
        assert_eq!(merge_subscription(Some(low.clone()), high), low);
    }

    #[test]
    fn threshold_is_inclusive() {
        let sub = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_e8s: 1_000_000,
        };
        assert!(!sub.matches(999_999));
        assert!(sub.matches(1_000_000));
        assert!(sub.matches(1_000_001));
    }

    #[test]
    fn omitted_threshold_matches_every_transfer_amount() {
        let sub = Subscription {
            subscriber: p(),
            numbered_subaccount: 7,
            minimum_e8s: 0,
        };
        assert!(sub.matches(0));
        assert!(sub.matches(1));
        assert!(sub.matches(u64::MAX));
    }
}
