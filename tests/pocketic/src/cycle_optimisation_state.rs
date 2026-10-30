use std::collections::BTreeMap;
type StableSnapshot = BTreeMap<u8, Vec<(Vec<u8>, Vec<u8>)>>;
// Independent host views of the documented stable schema. These read a copy of
// PocketIC memory through the pinned SDK; they never mutate a canister and never
// compare physical B-tree pages/allocation history.
fn effective_stable_snapshot(env: &Env) -> StableSnapshot {
    use ic_stable_structures::{
        memory_manager::{MemoryId, MemoryManager},
        storable::Bound,
        StableBTreeMap, StableCell, Storable, VectorMemory,
    };
    use std::{borrow::Cow, cell::RefCell, rc::Rc};
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Bytes<const N: u32>(Vec<u8>);
    impl<const N: u32> Storable for Bytes<N> {
        fn to_bytes(&self) -> Cow<'_, [u8]> {
            Cow::Borrowed(&self.0)
        }
        fn into_bytes(self) -> Vec<u8> {
            self.0
        }
        fn from_bytes(b: Cow<'_, [u8]>) -> Self {
            Self(b.into_owned())
        }
        const BOUND: Bound = Bound::Bounded {
            max_size: N,
            is_fixed_size: false,
        };
    }
    let memory: VectorMemory = Rc::new(RefCell::new(env.pic.get_stable_memory(env.event_horizon)));
    let manager = MemoryManager::init(memory);
    let mut out = BTreeMap::new();
    macro_rules! cell {
        ($id:expr,$n:expr) => {{
            let cell = StableCell::init(manager.get(MemoryId::new($id)), Bytes::<$n>(vec![]));
            out.insert($id, vec![(vec![], cell.get().0.clone())]);
        }};
    }
    for id in [0u8, 2, 3, 8, 10] {
        cell!(id, 256);
    }
    for id in [6u8, 7] {
        cell!(id, 512);
    }
    let self_map = StableBTreeMap::<Bytes<63>, Bytes<256>, _>::init(manager.get(MemoryId::new(1)));
    out.insert(
        1,
        self_map
            .iter()
            .map(|e| (e.key().0.clone(), e.value().0))
            .collect(),
    );
    let globals = StableBTreeMap::<Bytes<29>, u8, _>::init(manager.get(MemoryId::new(4)));
    out.insert(
        4,
        globals
            .iter()
            .map(|e| (e.key().0.clone(), vec![e.value()]))
            .collect(),
    );
    let observations = StableBTreeMap::<u64, Bytes<128>, _>::init(manager.get(MemoryId::new(5)));
    out.insert(
        5,
        observations
            .iter()
            .map(|e| (e.key().to_be_bytes().to_vec(), e.value().0))
            .collect(),
    );
    let aliases = StableBTreeMap::<String, Principal, _>::init(manager.get(MemoryId::new(9)));
    out.insert(
        9,
        aliases
            .iter()
            .map(|e| (e.key().as_bytes().to_vec(), e.value().as_slice().to_vec()))
            .collect(),
    );
    let external =
        StableBTreeMap::<Bytes<134>, Bytes<512>, _>::init(manager.get(MemoryId::new(11)));
    out.insert(
        11,
        external
            .iter()
            .map(|e| (e.key().0.clone(), e.value().0))
            .collect(),
    );
    out
}
fn assert_effective_snapshots_equal(old: &StableSnapshot, new: &StableSnapshot) -> Result<()> {
    assert_eq!(
        old.keys().collect::<Vec<_>>(),
        new.keys().collect::<Vec<_>>()
    );
    for (id, rows) in old {
        if rows != &new[id] {
            bail!("effective stable region{id} differs: baseline_rows={} candidate_rows={} baseline_digest={:x} candidate_digest={:x}",rows.len(),new[id].len(),Sha256::digest(encode_one(rows)?),Sha256::digest(encode_one(&new[id])?));
        }
    }
    Ok(())
}
fn complete_stable_differential(generic: bool) -> Result<()> {
    let old = baseline()?;
    let new = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
    let mut reference = None;
    for bytes in [old, new] {
        let env = Env::with_anchors_at(
            bytes,
            200_000_000_000_000,
            true,
            generic,
            generic,
            false,
            Some(pocket_ic::Time::from_nanos_since_unix_epoch(
                1_700_000_000_000_000_000,
            )),
        )?;
        env.price_once()?;
        env.poll()?;
        set_aliases(
            &env,
            vec![
                CanisterAlias {
                    alias: "A".into(),
                    canister_id: env.subscriber,
                },
                CanisterAlias {
                    alias: "AA".into(),
                    canister_id: env.subscriber,
                },
                CanisterAlias {
                    alias: "B".into(),
                    canister_id: env.cmc,
                },
                CanisterAlias {
                    alias: "C".into(),
                    canister_id: env.subscriber_second,
                },
                CanisterAlias {
                    alias: "E".into(),
                    canister_id: env.event_horizon,
                },
            ],
            "",
            None,
        )?;
        refresh_aliases(&env)?;
        for memo in ["A.0-255:0.02", "A@B.0-255:0.03", "C@B.7:1", "A.n0-1:0.01"] {
            fund_suffix(&env, memo)?;
        }
        env.poll()?;
        env.admit_global(100_000 * ICP_E8S)?;
        let first = effective_stable_snapshot(&env);
        assert_eq!(first[&1].len(), 258);
        assert_eq!(first[&11].len(), 257);
        assert_eq!(first[&4].len(), 1);
        assert_eq!(first[&9].len(), 5);
        // A high amount qualifies A but not C. Lowering C's threshold in this
        // page invalidates the high-water generation before its later firstmatch.
        let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
        env.append_observed(
            source.clone(),
            account_id(env.cmc, numbered(7)),
            50_000_000,
            None,
        )?;
        // Shared admission precedes a smaller event; generic observation remains
        // before its independent admission stream, so the next poll supplies it.
        fund_suffix(&env, "C@B.7:0.01")?;
        env.append_observed(
            source.clone(),
            account_id(env.cmc, numbered(7)),
            2_000_000,
            None,
        )?;
        env.poll()?;
        env.append_observed(
            source.clone(),
            account_id(env.cmc, numbered(7)),
            2_000_000,
            None,
        )?;
        fund_suffix(&env, "AA@B.7:0.04")?;
        fund_suffix(&env, "A.0-255:0.02")?;
        env.poll()?;
        // Exact integer threshold boundary, not merely preview of the memo.
        env.append_observed(
            source.clone(),
            account_id(env.cmc, numbered(0)),
            2_999_999,
            None,
        )?;
        let at = env.append_observed(source, account_id(env.cmc, numbered(0)), 3_000_000, None)?;
        let before = poke_count(&env, env.subscriber)?;
        env.poll()?;
        env.wait_for_wakeup_pokes(env.subscriber, before + 1)?;
        assert_eq!(
            notification(&env, env.subscriber)?.matches,
            vec![PokeMatch {
                subject: env.cmc,
                target: WatchTarget::Subaccount(0),
                first_block_index: at,
                max_amount: 3_000_000u64.into()
            }]
        );
        let second = effective_stable_snapshot(&env);
        // Stage a representative durable journal under identical supplied
        // liquidity, with fixed timestamps at the financial decision point.
        env.pic
            .set_time(pocket_ic::Time::from_nanos_since_unix_epoch(
                1_700_000_100_000_000_000,
            ));
        // The debug level setter deliberately samples real liquid cycles. Fund
        // next with a lower, identical supplied balance so observe() records
        // the controlled 150T minimum in both implementations. This preserves
        // comparison of the entire financial record despite different costs.
        env.set_liquid_cycles_override(Some(150_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert!(env
            .state()?
            .cmc_state
            .contains("planned_surplus: Some(PlannedSurplus"));
        let third = effective_stable_snapshot(&env);
        if let Some((a, b, c)) = &reference {
            assert_effective_snapshots_equal(a, &first)?;
            assert_effective_snapshots_equal(b, &second)?;
            assert_effective_snapshots_equal(c, &third)?;
        } else {
            reference = Some((first, second, third));
        }
    }
    Ok(())
}
#[test]
#[ignore = "independent baseline/candidate logical contents of stable IDs0–11, ICP"]
fn independent_complete_stable_differential_icp() -> Result<()> {
    complete_stable_differential(false)
}
#[test]
#[ignore = "independent baseline/candidate logical contents of stable IDs0–11, generic"]
fn independent_complete_stable_differential_generic() -> Result<()> {
    complete_stable_differential(true)
}
