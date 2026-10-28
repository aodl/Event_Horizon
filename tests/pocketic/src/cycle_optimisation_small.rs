#[test]
#[ignore = "serial paired real self/external/neuron subscriber measurements"]
#[cfg(feature = "optimisation_benchmarks")]
fn small_fixed_work() -> Result<()> {
    use std::collections::BTreeMap;
    let old = baseline()?;
    let new = std::fs::read(std::env::var_os("EH_OPT_CANDIDATE").context("candidate required")?)?;
    let mut csv=String::from("variant,sample,generic,scenario,blocks,entries,sender_cycles,immediate_sender_cycles,attempts,accepted,cursor,legacy_calls,icrc3_calls,received,typed_payload_bytes,wasm_heap_before,wasm_heap_after,stable_bytes\n");
    for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
        for entries in [1u64, 16] {
            if !selected_scenario("small_mixed", entries, entries) {
                continue;
            }
            for sample in samples() {
                let mut reference = None;
                for (variant, bytes) in [("baseline", &old), ("candidate", &new)] {
                    let env = Env::with_anchors_at(
                        bytes.clone(),
                        2_000_000_000_000_000,
                        false,
                        generic,
                        generic && entries == 16,
                        false,
                        Some(pocket_ic::Time::from_nanos_since_unix_epoch(
                            1_700_000_000_000_000_000,
                        )),
                    )?;
                    eprintln!("fixture topology={:?}", env.pic.topology());
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
                                alias: "B".into(),
                                canister_id: env.cmc,
                            },
                            CanisterAlias {
                                alias: "C".into(),
                                canister_id: env.subscriber_second,
                            },
                        ],
                        "",
                        None,
                    )?;
                    refresh_aliases(&env)?;
                    if entries == 1 {
                        fund_suffix(&env, "A.0:0.01")?;
                    } else {
                        for memo in ["A.0-3:0.01", "A@B.0-3:0.01", "A.n0-3:0.01", "C@B.n4-7:0.01"] {
                            fund_suffix(&env, memo)?;
                        }
                    }
                    env.poll()?;
                    assert_eq!(
                        env.state()?.subscriptions,
                        if entries == 1 { 1 } else { 8 },
                        "fixture self relations not admitted (external relations verified by exact hints)"
                    );
                    let profile: InstanceInfo =
                        query(&env.pic, env.event_horizon, "get_instance", ())?;
                    let governance = profile.observed_profile.and_then(|p| p.neuron_governance);
                    let start = env.state()?.observed_next_block;
                    let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                    let mut expected =
                        BTreeMap::<Principal, BTreeMap<(Principal, WatchTarget), PokeMatch>>::new();
                    let mut batch = Vec::new();
                    for i in 0..entries {
                        let (subscriber, subject, target, to) = match i {
                            0..=3 => (
                                env.subscriber,
                                env.subscriber,
                                WatchTarget::Subaccount(i),
                                account_id(env.subscriber, numbered(i)),
                            ),
                            4..=7 => (
                                env.subscriber,
                                env.cmc,
                                WatchTarget::Subaccount(i - 4),
                                account_id(env.cmc, numbered(i - 4)),
                            ),
                            8..=11 => (
                                env.subscriber,
                                env.subscriber,
                                WatchTarget::NeuronNonce(i - 8),
                                account_id(
                                    governance.context("missing verified governance")?,
                                    neuron_staking(env.subscriber, i - 8),
                                ),
                            ),
                            _ => (
                                env.subscriber_second,
                                env.cmc,
                                WatchTarget::NeuronNonce(i - 8),
                                account_id(
                                    governance.context("missing verified governance")?,
                                    neuron_staking(env.cmc, i - 8),
                                ),
                            ),
                        };
                        expected.entry(subscriber).or_default().insert(
                            (subject, target),
                            PokeMatch {
                                subject,
                                target,
                                first_block_index: start + i,
                                max_amount: 2_000_000u64.into(),
                            },
                        );
                        batch.push(Append {
                            from: source.clone(),
                            to,
                            amount_e8s: 2_000_000,
                            icrc1_memo: None,
                        });
                    }
                    update::<_, u64>(
                        &env.pic,
                        env.observed_ledger,
                        "debug_append_transfers",
                        batch,
                    )?;
                    for _ in 0..70 {
                        env.pic.tick();
                    }
                    let before_heap = env
                        .pic
                        .canister_status(env.event_horizon, None)
                        .map_err(|e| anyhow!("{e:?}"))?
                        .memory_metrics
                        .wasm_memory_size
                        .0;
                    let calls = || -> Result<(u64, u64)> {
                        Ok((
                            query(
                                &env.pic,
                                env.event_horizon,
                                "debug_legacy_query_blocks_calls",
                                (),
                            )?,
                            query(
                                &env.pic,
                                env.event_horizon,
                                "debug_icrc3_get_blocks_calls",
                                (),
                            )?,
                        ))
                    };
                    let before_calls = calls()?;
                    let before_pokes = env.poke_counts()?;
                    let before = env.pic.cycle_balance(env.event_horizon);
                    env.poll()?;
                    for _ in 0..40 {
                        env.pic.tick();
                    }
                    let immediate = before
                        .checked_sub(env.pic.cycle_balance(env.event_horizon))
                        .context("sender credit")?;
                    env.pic.advance_time(std::time::Duration::from_secs(301));
                    // This real-receiver fixture's periodic storage debit is
                    // applied after round 40 (diagnostic trace: round 50). Drain
                    // 120 fixed rounds identically, then require quiescence.
                    for _ in 0..120 {
                        env.pic.tick();
                    }
                    let settled = env.pic.cycle_balance(env.event_horizon);
                    for _ in 0..10 {
                        env.pic.tick();
                    }
                    assert_eq!(
                        settled,
                        env.pic.cycle_balance(env.event_horizon),
                        "unsettled poll"
                    );
                    let used = before.checked_sub(settled).context("settled credit")?;
                    let state = env.state()?;
                    assert_eq!(state.observed_next_block, start + entries);
                    let after_calls = calls()?;
                    let delta = (
                        after_calls.0 - before_calls.0,
                        after_calls.1 - before_calls.1,
                    );
                    let after_pokes = env.poke_counts()?;
                    let count = expected.len() as u64;
                    assert_eq!(after_pokes[0] - before_pokes[0], count);
                    assert_eq!(after_pokes[1] - before_pokes[1], count);
                    let mut actual = Vec::new();
                    let mut payload = 0usize;
                    for (subscriber, hints) in expected {
                        env.wait_for_wakeup_pokes(subscriber, 1)?;
                        assert_eq!(
                            query::<_, Option<Principal>>(
                                &env.pic,
                                subscriber,
                                "debug_last_caller",
                                ()
                            )?,
                            Some(env.event_horizon)
                        );
                        let n = notification(&env, subscriber)?;
                        assert_eq!(
                            n,
                            PokeNotification {
                                global_activity: false,
                                matches: hints.into_values().collect()
                            }
                        );
                        payload += encode_one(&n)?.len();
                        actual.push((subscriber, n));
                    }
                    let semantic = encode_one((&state, delta, actual))?;
                    if let Some(ref previous) = reference {
                        assert_eq!(previous, &semantic, "small mixed differential");
                    } else {
                        reference = Some(semantic);
                    }
                    let status = env
                        .pic
                        .canister_status(env.event_horizon, None)
                        .map_err(|e| anyhow!("{e:?}"))?;
                    csv.push_str(&format!("{variant},{sample},{generic},small_mixed,{entries},{entries},{used},{immediate},{count},{count},{entries},{},{},{count},{payload},{before_heap},{},{}\n",delta.0,delta.1,status.memory_metrics.wasm_memory_size.0,status.memory_metrics.stable_memory_size.0));
                }
            }
        }
    }
    save_complete("EH_OPT_OUTPUT", &csv)?;
    println!("{csv}");
    Ok(())
}
