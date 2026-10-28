//! Independent linear oracle: deliberately does not use record_match or Subscription::matches.
use super::*;

fn p(n: u64) -> Principal {
    let mut b = n.to_be_bytes().to_vec();
    b.push(1);
    Principal::from_slice(&b)
}
fn relation(n: u64, subject: u64, threshold: u64) -> SubjectSubscription {
    SubjectSubscription {
        subject: p(subject),
        subscription: Subscription {
            subscriber: p(1),
            target: WatchTarget::Subaccount(n),
            minimum_units: threshold.into(),
        },
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    subscriber: Principal,
    subject: Principal,
    target: WatchTarget,
    first: u64,
    maximum: Nat,
}
fn reference(rows: &mut Vec<Row>, watchers: &[SubjectSubscription], index: u64, amount: &Nat) {
    for w in watchers {
        if amount < &w.subscription.minimum_units {
            continue;
        }
        if let Some(row) = rows.iter_mut().find(|r| {
            r.subscriber == w.subscription.subscriber
                && r.subject == w.subject
                && r.target == w.subscription.target
        }) {
            if index < row.first {
                row.first = index;
            }
            if amount > &row.maximum {
                row.maximum = amount.clone();
            }
        } else if rows
            .iter()
            .filter(|r| r.subscriber == w.subscription.subscriber)
            .count()
            < 256
        {
            rows.push(Row {
                subscriber: w.subscription.subscriber,
                subject: w.subject,
                target: w.subscription.target,
                first: index,
                maximum: amount.clone(),
            });
        }
    }
}
fn check(rows: &[Row], actual: &BTreeMap<Principal, MatchState>, seed: u64, step: usize) {
    let expected: BTreeMap<_, _> = rows
        .iter()
        .map(|r| {
            (
                (r.subscriber, r.subject, r.target),
                (r.first, r.maximum.clone()),
            )
        })
        .collect();
    let actual: BTreeMap<_, _> = actual
        .iter()
        .flat_map(|(p, s)| {
            s.subs.iter().map(move |((subject, target), m)| {
                (
                    (*p, *subject, *target),
                    (m.first_block_index, m.max_amount.clone()),
                )
            })
        })
        .collect();
    assert_eq!(actual, expected, "seed={seed} step={step}");
}
#[test]
fn record_high_matches_independent_randomized_oracle() {
    for seed in 0..32u64 {
        let mut rng = seed + 1;
        let mut watchers: Vec<_> = (0..4)
            .map(|account| {
                (0..300)
                    .map(|n| relation(n, account + 10, (n % 4) * 30))
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut caches = BTreeMap::<usize, CachedInterests>::new();
        let mut actual = BTreeMap::new();
        let mut expected = Vec::new();
        for step in 0..512usize {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let destination = ((rng >> 32) % 4) as usize;
            if step % 32 == 0 {
                caches.clear();
            } // Page boundary.
            if step % 19 == 0 {
                // More permissive effective relation: new generation.
                let w = &mut watchers[destination][((rng >> 8) % 300) as usize];
                w.subscription.minimum_units = 0u8.into();
                caches.remove(&destination);
            }
            if step % 23 == 0 {
                // Newly admitted effective identity must not inherit a prior
                // destination high-water, including under a different subscriber.
                let mut added = relation(300 + step as u64, destination as u64 + 10, rng % 2000);
                added.subscription.subscriber = p(2 + (destination % 2) as u64);
                if step % 2 == 0 {
                    added.subscription.target = WatchTarget::NeuronNonce(300 + step as u64);
                }
                watchers[destination].push(added);
                caches.remove(&destination);
            }
            let amount = match step % 9 {
                0 => 0u8.into(),
                1 => 1u8.into(),
                2 => candid::Nat::from(20u8),
                3 => candid::Nat::from(20u8),
                4 => Nat(Nat::from(1u8).0 << 256usize),
                5 => Nat(Nat::from(1u8).0 << 4096usize),
                6 => Nat(Nat::from(1u8).0 << 32768usize),
                _ => Nat::from(rng % 1000),
            };
            let index = u64::MAX - 1024 + step as u64;
            reference(&mut expected, &watchers[destination], index, &amount);
            caches
                .entry(destination)
                .or_insert_with(|| CachedInterests::new(watchers[destination].clone()))
                .observe(index, &amount, &mut actual);
            check(&expected, &actual, seed, step);
        }
    }
}
#[test]
fn threshold_reduction_invalidates_old_high_water() {
    let mut out = BTreeMap::new();
    let mut expected = Vec::new();
    let high = vec![relation(7, 2, 2000), relation(8, 2, 2000)];
    let mut cache = CachedInterests::new(high.clone());
    cache.observe(10, &1000u16.into(), &mut out);
    reference(&mut expected, &high, 10, &1000u16.into());
    let low = vec![relation(7, 2, 1), relation(8, 2, 2000)];
    cache = CachedInterests::new(low.clone());
    cache.observe(11, &10u8.into(), &mut out);
    reference(&mut expected, &low, 11, &10u8.into());
    check(&expected, &out, 0, 0);
    assert_eq!(
        out[&p(1)].subs[&(p(2), WatchTarget::Subaccount(7))].first_block_index,
        11
    );
}
#[test]
fn equal_transfers_walk_1024_watchers_once_and_empty_sets_keep_no_nat() {
    let mut cache = CachedInterests::new(
        (0..1024)
            .map(|n| SubjectSubscription {
                subject: p(2),
                subscription: Subscription {
                    subscriber: p(n + 10),
                    target: WatchTarget::Subaccount(7),
                    minimum_units: 0u8.into(),
                },
            })
            .collect(),
    );
    let mut out = BTreeMap::new();
    for i in 0..256 {
        cache.observe(i, &candid::Nat::from(20u8), &mut out);
    }
    assert_eq!(cache.walked, 1024);
    assert_eq!(out.len(), 1024);
    for s in out.values() {
        assert_eq!(s.subs.values().next().unwrap().first_block_index, 0);
    }
    let mut empty = CachedInterests::new(vec![]);
    empty.observe(1, &Nat(Nat::from(1u8).0 << 32768usize), &mut out);
    assert!(empty.record_high.is_none());
    assert_eq!(empty.walked, 0);
}

#[test]
fn empty_shared_pages_do_not_walk_globals_and_live_snapshot_is_once() {
    for count in [0, 256, 1024] {
        std::thread::spawn(move || {
            fn ready<F: std::future::Future>(f: F) -> F::Output {
                let mut f = std::pin::pin!(f);
                match f
                    .as_mut()
                    .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
                {
                    std::task::Poll::Ready(v) => v,
                    std::task::Poll::Pending => {
                        panic!("unexpected external call in no-memo fixture")
                    }
                }
            }
            state::modify_metadata(|m| {
                m.admission_bootstrapped = true;
                m.observed_bootstrapped = true;
            });
            for n in 0..count {
                state::put_global_subscriber(p(n + 10));
            }
            let mut order = SharedGlobalOrder {
                existing: None,
                admitted_at: BTreeMap::new(),
                last_live: None,
            };
            let mut context = AdmissionContext {
                runtime: config::RuntimeConfig::production(p(8), None),
                own_id: p(9),
                decimals: 8,
                accounts: None,
                relations: None,
            };
            let mut out = BTreeMap::new();
            let empty = icp_ledger::QueryBlocksResponse {
                chain_length: 0,
                blocks: vec![],
                first_block_index: u64::MAX,
                archived_blocks: vec![],
            };
            assert_eq!(
                ready(legacy_page(
                    empty,
                    Stream::Shared,
                    0,
                    0..0,
                    &mut context,
                    &mut out,
                    Some(&mut order)
                ))
                .unwrap(),
                (0, false)
            );
            assert_eq!(state::test_global_visits(), 0);
            assert!(order.existing.is_none());
            for index in 0..2 {
                let response = icp_ledger::QueryBlocksResponse {
                    chain_length: index + 1,
                    first_block_index: index,
                    archived_blocks: vec![],
                    blocks: vec![icp_ledger::Block {
                        parent_hash: None,
                        timestamp: icp_ledger::TimeStamp { timestamp_nanos: 0 },
                        transaction: icp_ledger::Transaction {
                            memo: 0,
                            icrc1_memo: None,
                            operation: None,
                            created_at_time: icp_ledger::TimeStamp { timestamp_nanos: 0 },
                        },
                    }],
                };
                assert_eq!(
                    ready(legacy_page(
                        response,
                        Stream::Shared,
                        index + 1,
                        index..index + 1,
                        &mut context,
                        &mut out,
                        Some(&mut order)
                    ))
                    .unwrap(),
                    (index + 1, true)
                );
                assert_eq!(state::test_global_visits(), count);
                assert_eq!(state::global_subscription_count(), count);
                assert_eq!(
                    state::test_global_visits(),
                    count,
                    "count must not enumerate"
                );
            }
        })
        .join()
        .unwrap();
    }
}

#[test]
fn duplicate_stage_keeps_pending_improvement_and_cache_generation() {
    std::thread::spawn(|| {
        let runtime = config::RuntimeConfig::production(p(8), None);
        let ledger = runtime.icp_ledger;
        state::initialize_instance_config(crate::instance::InstanceConfig {
            observed_ledger: ledger,
            observed_profile: None,
            sns_root: None,
            surplus_canister: None,
        });
        let make = |minimum| {
            SubjectSubscription::self_subject(Subscription {
                subscriber: p(1),
                target: WatchTarget::Subaccount(7),
                minimum_units: Nat::from(minimum),
            })
        };
        state::put_relation(make(100u64));
        state::put_relation(SubjectSubscription {
            subject: p(1),
            subscription: Subscription {
                subscriber: p(2),
                target: WatchTarget::Subaccount(7),
                minimum_units: 0u8.into(),
            },
        });
        let account = account_identifier_bytes(p(1), crate::account::numbered_subaccount(7));
        let mut pending = PendingAdmissions::default();
        let mut out = BTreeMap::new();
        pending
            .shared_subscriptions(account)
            .observe(0, &150u16.into(), &mut out);
        let mut context = AdmissionContext {
            runtime: config::RuntimeConfig::production(ledger, None),
            own_id: p(9),
            decimals: 8,
            accounts: None,
            relations: None,
        };
        pending.stage(PreparedAdmission::Accounts(vec![make(20u64)]), &mut context);
        assert!(!pending.shared_cache.contains_key(&account));
        pending
            .shared_subscriptions(account)
            .observe(1, &10u8.into(), &mut out);
        pending.stage(
            PreparedAdmission::Accounts(vec![make(50u64), make(20u64)]),
            &mut context,
        );
        assert_eq!(
            pending.shared_cache[&account].record_high,
            Some(10u8.into())
        );
        let writes = state::test_stable_writes()[0];
        pending.publish();
        assert_eq!(state::test_stable_writes()[0], writes + 1);
        assert_eq!(
            state::get_relation(p(1), p(1), WatchTarget::Subaccount(7))
                .unwrap()
                .subscription
                .minimum_units,
            candid::Nat::from(20u8)
        );
    })
    .join()
    .unwrap();
}

#[test]
fn unchanged_stable_declarations_reuse_one_key_and_do_not_publish_or_invalidate() {
    std::thread::spawn(|| {
        let ledger = config::RuntimeConfig::production(p(8), None).icp_ledger;
        state::initialize_instance_config(crate::instance::InstanceConfig {
            observed_ledger: ledger,
            observed_profile: None,
            sns_root: None,
            surplus_canister: None,
        });
        let make = |minimum| {
            SubjectSubscription::self_subject(Subscription {
                subscriber: p(1),
                target: WatchTarget::Subaccount(7),
                minimum_units: Nat::from(minimum),
            })
        };
        state::put_relation(make(100u64));
        state::put_relation(SubjectSubscription {
            subject: p(1),
            subscription: Subscription {
                subscriber: p(2),
                target: WatchTarget::Subaccount(7),
                minimum_units: 0u8.into(),
            },
        });
        let account = account_identifier_bytes(p(1), crate::account::numbered_subaccount(7));
        let mut pending = PendingAdmissions::default();
        pending
            .shared_subscriptions(account)
            .observe(0, &150u16.into(), &mut BTreeMap::new());
        let mut context = AdmissionContext {
            runtime: config::RuntimeConfig::production(ledger, None),
            own_id: p(9),
            decimals: 8,
            accounts: None,
            relations: None,
        };
        let derivations = state::test_key_derivations();
        let writes = state::test_stable_writes();
        for i in 0..256 {
            pending.stage(
                PreparedAdmission::Accounts(vec![make(100u64 + i % 2)]),
                &mut context,
            );
        }
        assert_eq!(state::test_key_derivations() - derivations, 1);
        assert_eq!(pending.subscriptions.len(), 1);
        assert!(!pending.subscriptions.values().next().unwrap().2);
        assert!(pending.shared_overlay.is_empty());
        assert_eq!(
            pending.shared_cache[&account].record_high,
            Some(150u16.into())
        );
        pending.publish();
        assert_eq!(state::test_stable_writes(), writes);
    })
    .join()
    .unwrap();
}
