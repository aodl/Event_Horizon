// Publication costs are measured through ordinary admission/alias code, with
// Historian evidence still independently requested for every exact memo.
#[test]
#[ignore = "serial paired publication measurements"]
#[cfg(feature = "optimisation_benchmarks")]
fn publication_fixed_work() -> Result<()> {
    let instrumented = std::env::var_os("EH_OPT_METRICS_OUTPUT").is_some();
    let mut metrics_csv = String::from(
        "variant,sample,generic,scenario,phase,calls,instructions,linear_heap_pages\n",
    );
    let old = baseline()?;
    let new = std::fs::read(
        std::env::var_os("EH_OPT_CANDIDATE").context("candidate artifact required")?,
    )?;
    let cases = [
        ("noop_self", 256, 1),
        ("noop_external", 256, 1),
        ("noop_global", 256, 1),
        ("new_global", 1, 1),
        ("threshold_lower", 256, 1),
        ("new_range", 1, 256),
        ("fullwidth_range", 1, 256),
        ("alias_unchanged", 0, 2),
        ("alias_unchanged", 0, 4096),
        ("alias_additions", 0, 2),
        ("alias_additions", 0, 4096),
    ];
    let mut csv=String::from("variant,sample,generic,scenario,blocks,entries,sender_cycles,immediate_sender_cycles,attempts,accepted,cursor,legacy_calls,icrc3_calls,alias_calls,validated_alias_pages,stable_bytes,total_bytes,ledger_cycles,historian_cycles,recipient_cycles,wasm_heap_before,wasm_heap_after\n");
    for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
        for (case, blocks, entries) in cases {
            if !selected_scenario(case, blocks, entries) {
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
                        false,
                        false,
                        Some(pocket_ic::Time::from_nanos_since_unix_epoch(
                            1_700_000_000_000_000_000,
                        )),
                    )?;
                    eprintln!("fixture topology={:?}", env.pic.topology());
                    env.price_once()?;
                    env.poll()?;
                    let alias_case = case.starts_with("alias_");
                    let compact = env.subscriber.to_text().replace('-', "");
                    let mut memo = Vec::new();
                    let mut second_memo = None;
                    let mut alias_items = Vec::new();
                    if alias_case {
                        alias_items = (0..entries - 1)
                            .map(|n| CanisterAlias {
                                alias: format!("A{n:04}"),
                                canister_id: env.subscriber,
                            })
                            .collect();
                        alias_items.push(CanisterAlias {
                            alias: "E".into(),
                            canister_id: env.event_horizon,
                        });
                        let mut initial = alias_items.clone();
                        if case == "alias_additions" {
                            initial.remove(initial.len() - 2);
                        }
                        set_aliases(&env, initial, "", None)?;
                        refresh_aliases(&env)?;
                        // A same-Wasm debug upgrade clears the automatically
                        // scheduled alias timer. Stored aliases/metadata survive;
                        // the due refresh below is then explicitly attributed.
                        env.upgrade_with_debug_wasm(bytes.clone())?;
                        assert!(preview(&env, "E.0")?.is_ok());
                        set_aliases(&env, alias_items.clone(), "", None)?;
                    } else {
                        match case {
                            "noop_self" => {
                                memo = env.admit(7, Some("0.01"), 10 * ICP_E8S)?;
                            }
                            "noop_external" => {
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
                                    ],
                                    "",
                                    None,
                                )?;
                                refresh_aliases(&env)?;
                                memo = b"A@B.7:0.01".to_vec();
                                fund_suffix(&env, std::str::from_utf8(&memo)?)?;
                                env.poll()?;
                            }
                            "new_global" => {
                                memo = compact.as_bytes().to_vec();
                                env.set_route(memo.clone(), GLOBAL_BASE_E8S)?;
                            }
                            "noop_global" => {
                                memo = env.admit_global(GLOBAL_BASE_E8S)?;
                            }
                            "threshold_lower" => {
                                env.admit(7, Some("1"), 10 * ICP_E8S)?;
                                memo = format!("{compact}.7:0.01").into_bytes();
                                let duplicate = format!("{compact}.7:0.02").into_bytes();
                                env.set_route(duplicate.clone(), 10 * ICP_E8S)?;
                                second_memo = Some(duplicate);
                                env.set_route(memo.clone(), 10 * ICP_E8S)?;
                            }
                            "new_range" | "fullwidth_range" => {
                                let start = if case == "new_range" {
                                    0
                                } else {
                                    u64::MAX - 255
                                };
                                memo = format!("{compact}.{start}-{}", start + 255).into_bytes();
                                env.set_route(memo.clone(), 20 * ICP_E8S)?;
                            }
                            _ => unreachable!(),
                        }
                    }
                    let initial = env.state()?;
                    if !alias_case {
                        let batch = (0..blocks)
                            .map(|i| Append {
                                from: account_id(env.faucet, [0; 32]),
                                to: account_id(env.event_horizon, [0; 32]),
                                amount_e8s: 1,
                                icrc1_memo: Some(
                                    if i > 0 {
                                        second_memo.as_ref().unwrap_or(&memo)
                                    } else {
                                        &memo
                                    }
                                    .clone(),
                                ),
                            })
                            .collect::<Vec<_>>();
                        update::<_, u64>(&env.pic, env.ledger, "debug_append_transfers", batch)?;
                    }
                    env.pic
                        .advance_time(std::time::Duration::from_secs(if alias_case {
                            86_400
                        } else {
                            1
                        }));
                    for _ in 0..70 {
                        env.pic.tick();
                    }
                    let settled = env.pic.cycle_balance(env.event_horizon);
                    for _ in 0..10 {
                        env.pic.tick();
                    }
                    assert_eq!(
                        settled,
                        env.pic.cycle_balance(env.event_horizon),
                        "setup not settled"
                    );
                    let call_counts = || -> Result<(u64, u64, u64)> {
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
                            query(&env.pic, env.historian, "debug_alias_calls", ())?,
                        ))
                    };
                    if instrumented {
                        update::<_, ()>(
                            &env.pic,
                            env.event_horizon,
                            "debug_reset_optimisation_metrics",
                            (),
                        )?;
                    }
                    let before_calls = call_counts()?;
                    let before_pokes = env.poke_counts()?;
                    let deps = [env.ledger, env.historian, env.subscriber];
                    let before_deps = deps.map(|p| env.pic.cycle_balance(p));
                    let memory_before = env
                        .pic
                        .canister_status(env.event_horizon, None)
                        .map_err(|e| anyhow!("{e:?}"))?
                        .memory_metrics
                        .wasm_memory_size
                        .0;
                    let before = env.pic.cycle_balance(env.event_horizon);
                    if alias_case {
                        refresh_aliases(&env)?;
                    } else {
                        env.poll()?;
                    }
                    for _ in 0..40 {
                        env.pic.tick();
                    }
                    let immediate = before
                        .checked_sub(env.pic.cycle_balance(env.event_horizon))
                        .context("unexpected credit")?;
                    let expected = u64::from(case == "noop_global" && !generic);
                    if expected > 0 {
                        env.pic.advance_time(std::time::Duration::from_secs(301));
                        for _ in 0..40 {
                            env.pic.tick();
                        }
                    }
                    let settled = env.pic.cycle_balance(env.event_horizon);
                    for _ in 0..10 {
                        env.pic.tick();
                    }
                    assert_eq!(
                        settled,
                        env.pic.cycle_balance(env.event_horizon),
                        "publication not settled"
                    );
                    let used = before
                        .checked_sub(settled)
                        .context("unexpected settled credit")?;
                    let dependency_used = deps
                        .iter()
                        .zip(before_deps)
                        .map(|(p, b)| {
                            b.checked_sub(env.pic.cycle_balance(*p))
                                .expect("dependency credit")
                        })
                        .collect::<Vec<_>>();
                    if instrumented {
                        let metrics: Vec<Metric> = query(
                            &env.pic,
                            env.event_horizon,
                            "debug_optimisation_metrics",
                            (),
                        )?;
                        assert!(!metrics.is_empty());
                        if alias_case {
                            assert_eq!(
                                metrics
                                    .iter()
                                    .find(|m| m.phase == "alias_request")
                                    .context("missing caller-side alias counter")?
                                    .calls,
                                entries.div_ceil(128)
                            );
                        }
                        for m in metrics {
                            metrics_csv.push_str(&format!(
                                "{variant},{sample},{generic},{case},{},{},{},{}\n",
                                m.phase, m.calls, m.instructions, m.linear_heap_pages
                            ));
                        }
                    }
                    let state = env.state()?;
                    assert_eq!(
                        state.admission_next_block,
                        initial.admission_next_block + blocks
                    );
                    assert_eq!(
                        state.observed_next_block,
                        initial.observed_next_block + if generic { 0 } else { blocks }
                    );
                    let after_calls = call_counts()?;
                    let calls = (
                        after_calls.0 - before_calls.0,
                        after_calls.1 - before_calls.1,
                        after_calls.2 - before_calls.2,
                    );
                    // Replicated query-local mock mutations are discarded.
                    // Its counter cannot provide positive request counts. Caller
                    // instrumentation provides them in separate shadow runs.

                    let after_pokes = env.poke_counts()?;
                    assert_eq!(after_pokes[0] - before_pokes[0], expected);
                    assert_eq!(after_pokes[1] - before_pokes[1], expected);
                    if expected > 0 {
                        env.wait_for_wakeup_pokes(env.subscriber, 1)?;
                        let n = notification(&env, env.subscriber)?;
                        assert!(n.global_activity && n.matches.is_empty());
                    }
                    let alias_metadata: AliasMetadata =
                        query(&env.pic, env.event_horizon, "debug_alias_metadata", ())?;
                    if alias_case {
                        let now = env.pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
                        assert_eq!(
                            alias_metadata.last_success,
                            Some(now),
                            "refresh did not complete"
                        );
                        assert_eq!(alias_metadata.last_attempt, Some(now));
                        let mut digest = Sha256::new();
                        digest.update(b"JUPITER_CANISTER_ALIASES_V1\0");
                        digest.update((alias_items.len() as u32).to_be_bytes());
                        for item in &alias_items {
                            digest.update([item.alias.len() as u8]);
                            digest.update(item.alias.as_bytes());
                            digest.update([item.canister_id.as_slice().len() as u8]);
                            digest.update(item.canister_id.as_slice());
                        }
                        assert_eq!(alias_metadata.revision, Some(digest.finalize().to_vec()));
                    }
                    let effective = if alias_case {
                        encode_one(preview(
                            &env,
                            &format!("{}.7", alias_items.last().unwrap().alias),
                        )?)?
                    } else if case == "noop_external" {
                        // Inspect effective external threshold behaviour through
                        // genuine transfers, outside the measured publication.
                        let from = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                        env.append_observed(
                            from.clone(),
                            account_id(env.cmc, numbered(7)),
                            500_000,
                            None,
                        )?;
                        let first = env.append_observed(
                            from,
                            account_id(env.cmc, numbered(7)),
                            2_000_000,
                            None,
                        )?;
                        env.poll()?;
                        env.wait_for_wakeup_pokes(env.subscriber, 1)?;
                        let n = notification(&env, env.subscriber)?;
                        assert_eq!(n.matches.len(), 1);
                        assert_eq!(n.matches[0].subject, env.cmc);
                        assert_eq!(n.matches[0].first_block_index, first);
                        assert_eq!(n.matches[0].max_amount, candid::Nat::from(2_000_000u64));
                        encode_one(n)?
                    } else if matches!(case, "noop_global" | "new_global") {
                        assert!(env.global_subscription()?);
                        vec![]
                    } else {
                        let target = if case == "fullwidth_range" {
                            u64::MAX
                        } else {
                            7
                        };
                        let relation = env
                            .subscription(target)?
                            .context("subscription not published")?;
                        if case == "threshold_lower" {
                            assert_eq!(relation.minimum_units, candid::Nat::from(1_000_000u64));
                        }
                        if case.ends_with("range") {
                            assert_eq!(state.subscriptions, 256);
                        }
                        encode_one(relation)?
                    };
                    let semantic = encode_one((&state, alias_metadata, effective, calls))?;
                    if let Some(ref reference) = reference {
                        assert_eq!(reference, &semantic, "publication differential {case}");
                    } else {
                        reference = Some(semantic);
                    }
                    let status = env
                        .pic
                        .canister_status(env.event_horizon, None)
                        .map_err(|e| anyhow!("{e:?}"))?;
                    csv.push_str(&format!("{variant},{sample},{generic},{case},{blocks},{entries},{used},{immediate},{expected},{expected},{blocks},{},{},,{},{},{},{},{},{},{},{}\n",calls.0,calls.1,if alias_case {entries.div_ceil(128)}else{0},status.memory_metrics.stable_memory_size.0,status.memory_size.0,dependency_used[0],dependency_used[1],dependency_used[2],memory_before,status.memory_metrics.wasm_memory_size.0));
                }
            }
        }
    }
    save_complete("EH_OPT_OUTPUT", &csv)?;
    if instrumented {
        save_complete("EH_OPT_METRICS_OUTPUT", &metrics_csv)?;
    }
    println!("{csv}");
    Ok(())
}
