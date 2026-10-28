// Host-only real-receiver production comparison. Fixture setup/inspection uses
// the verified baseline debug companion outside the measured interval.
#[test]
#[ignore = "real receivers against independently built production modules"]
#[cfg(feature = "optimisation_benchmarks")]
fn production_real_fixed_work() -> Result<()> {
    #[derive(CandidType, Deserialize)]
    struct RealRelation {
        subject: Principal,
        subscription: Subscription,
    }
    let root = workspace_root()?;
    let old = std::fs::read(root.join("tests/fixtures/event_horizon_81c5a0d_production.wasm"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&old)),
        "2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528"
    );
    let path = std::env::var_os("EH_OPT_CANDIDATE_PROD")
        .context("explicit production candidate required")?;
    let new = std::fs::read(&path)?;
    assert!(std::env::var_os("EH_OPT_METRICS_OUTPUT").is_none());
    let canonical = if let Ok(source) = std::env::var("EH_OPT_CANONICAL_SOURCE") {
        let head = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()?;
        assert!(head.status.success());
        assert_eq!(source, String::from_utf8(head.stdout)?.trim());
        assert!(Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&root)
            .output()?
            .stdout
            .is_empty());
        assert_eq!(
            std::fs::canonicalize(&path)?,
            std::fs::canonicalize(root.join("release-artifacts/event_horizon.wasm"))?
        );
        true
    } else {
        false
    };
    let seed = baseline()?;
    for (cache, package) in [
        (&LEDGER_WASM, "mock-icp-ledger"),
        (&ICRC3_LEDGER_WASM, "mock-icrc3-ledger"),
        (&HISTORIAN_WASM, "mock-historian"),
        (&CMC_WASM, "mock-cmc"),
        (&SUBSCRIBER_WASM, "mock-subscriber"),
    ] {
        wasm(cache, package, None)?;
    }
    let receiver_wasm = wasm(&SUBSCRIBER_WASM, "mock-subscriber", None)?;
    // A global-only call reserves 42,102,481,000 cycles in this pinned
    // environment. Ample actual liquidity covers 1,024 simultaneous calls and
    // keeps both variants in VeryFast even before response reservations refund.
    let initial_cycles = 80_000_000_000_000u128;
    let window_seconds = 10;
    let mut csv = String::from("variant,sample,generic,scenario,blocks,entries,sender_cycles,recipient_cycles,ledger_cycles,historian_cycles,callbacks,typed_payload_bytes,start_cursor,observed_cursor,admission_cursor,stable_bytes,wasm_heap_before,wasm_heap_after,canonical_candidate,idle_cycles_per_day,initial_cycles,window_seconds,polling_mode,sender_balance_before,sender_balance_after,excluded_fixture_credit\n");
    // Predeclared representative final-production matrix. The1024 row is an
    // optional explicitly selected scale smoke; default runner selects32.
    for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
        for (scenario, blocks, entries) in [
            ("real_equal", 256u64, 32u64),
            ("real_increasing", 256, 32),
            ("real_unmatched_increasing", 256, 32),
            ("real_global", 1, 256),
            ("real_growing", 4096, 512),
            ("real_equal", 256, 1024),
        ] {
            if !selected_scenario(scenario, blocks, entries) {
                continue;
            }
            for sample in samples() {
                let mut reference = None;
                for (variant, bytes) in [("baseline", &old), ("candidate", &new)] {
                    eprintln!("real production setup variant={variant} generic={generic} scenario={scenario} entries={entries}");
                    let env = Env::with_anchors(
                        seed.clone(),
                        initial_cycles,
                        false,
                        generic,
                        false,
                        true,
                    )?;
                    eprintln!("fixture topology={:?}", env.pic.topology());
                    env.price_once()?;
                    env.poll()?;
                    refresh_aliases(&env)?;
                    let receivers = if scenario == "real_growing" {
                        env.admit_range(0, 255, None, 20 * ICP_E8S)?;
                        env.admit_range(256, 511, None, 20 * ICP_E8S)?;
                        vec![env.subscriber]
                    } else {
                        let mut receivers = Vec::new();
                        let mut payouts = Vec::new();
                        for _ in 0..entries {
                            let id = env.pic.create_canister();
                            env.pic.add_cycles(id, 200_000_000_000_000);
                            env.pic
                                .install_canister(id, receiver_wasm.clone(), vec![], None);
                            let compact = id.to_text().replace('-', "");
                            let memo = if scenario == "real_global" {
                                compact.into_bytes()
                            } else {
                                let threshold = if scenario == "real_unmatched_increasing" {
                                    ":100"
                                } else {
                                    ""
                                };
                                format!(
                                    "{compact}@{}.7{threshold}",
                                    env.subscriber.to_text().replace('-', "")
                                )
                                .into_bytes()
                            };
                            env.set_route(
                                memo.clone(),
                                if scenario == "real_global" {
                                    GLOBAL_BASE_E8S
                                } else {
                                    10 * ICP_E8S
                                },
                            )?;
                            payouts.push(Append {
                                from: account_id(env.faucet, [0; 32]),
                                to: account_id(env.event_horizon, [0; 32]),
                                amount_e8s: 1,
                                icrc1_memo: Some(memo),
                            });
                            receivers.push(id);
                            // Fixture-only admission batches keep baseline setup
                            // below PocketIC's 100-round ingress wait. Production
                            // work is measured only after all setup settles.
                            if payouts.len() == 16 {
                                update::<_, u64>(
                                    &env.pic,
                                    env.ledger,
                                    "debug_append_transfers",
                                    std::mem::take(&mut payouts),
                                )?;
                                env.poll()?;
                            }
                        }
                        if !payouts.is_empty() {
                            update::<_, u64>(
                                &env.pic,
                                env.ledger,
                                "debug_append_transfers",
                                payouts,
                            )?;
                            env.poll()?;
                        }
                        receivers
                    };
                    // Settle seed admissions and their genuine receiver calls.
                    for _ in 0..200 {
                        env.pic.tick();
                    }
                    let initial = env.state()?;
                    assert_eq!(
                        initial.global_subscriptions,
                        if scenario == "real_global" {
                            entries
                        } else {
                            0
                        }
                    );
                    if scenario == "real_growing" {
                        assert_eq!(initial.subscriptions, 512);
                    }
                    if !matches!(scenario, "real_global" | "real_growing") {
                        // Reject a fixture that failed to admit its watchers,
                        // especially the deliberately nonmatching threshold case.
                        let snapshot = effective_stable_snapshot(&env);
                        assert_eq!(snapshot[&11].len(), entries as usize);
                        let mut admitted = std::collections::BTreeSet::new();
                        for (_, value) in &snapshot[&11] {
                            let relation: RealRelation = candid::decode_one(value)?;
                            assert_eq!(relation.subject, env.subscriber);
                            assert_eq!(relation.subscription.target, WatchTarget::Subaccount(7));
                            assert_eq!(
                                relation.subscription.minimum_units,
                                candid::Nat::from(if scenario == "real_unmatched_increasing" {
                                    10_000_000_000u64
                                } else {
                                    0
                                })
                            );
                            admitted.insert(relation.subscription.subscriber);
                        }
                        assert_eq!(admitted, receivers.iter().copied().collect());
                    }
                    let receiver_counts = receivers
                        .iter()
                        .map(|id| poke_count(&env, *id))
                        .collect::<Result<Vec<_>>>()?;
                    env.pic
                        .upgrade_canister(
                            env.event_horizon,
                            bytes.clone(),
                            candid::encode_args(())?,
                            None,
                        )
                        .map_err(|e| anyhow!("production upgrade {e:?}"))?;
                    for _ in 0..200 {
                        env.pic.tick();
                    }
                    let settled = env.pic.cycle_balance(env.event_horizon);
                    for _ in 0..20 {
                        env.pic.tick();
                    }
                    assert_eq!(
                        settled,
                        env.pic.cycle_balance(env.event_horizon),
                        "initial lanes not settled"
                    );
                    for (id, count) in receivers.iter().zip(&receiver_counts) {
                        assert_eq!(
                            poke_count(&env, *id)?,
                            *count,
                            "setup generated an unexpected production notification"
                        );
                    }
                    let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                    let mut transfers = Vec::new();
                    for i in 0..blocks {
                        let amount = match scenario {
                            "real_increasing" | "real_unmatched_increasing" => i + 1,
                            "real_growing" if i + 1 == blocks => 1000,
                            _ => 20,
                        };
                        let target = if scenario == "real_growing" {
                            if i + 1 == blocks {
                                0
                            } else {
                                i
                            }
                        } else {
                            7
                        };
                        transfers.push(Append {
                            from: source.clone(),
                            to: account_id(env.subscriber, numbered(target)),
                            amount_e8s: amount,
                            icrc1_memo: None,
                        });
                        if transfers.len() == 256 {
                            update::<_, u64>(
                                &env.pic,
                                env.observed_ledger,
                                "debug_append_transfers",
                                std::mem::take(&mut transfers),
                            )?;
                        }
                    }
                    if !transfers.is_empty() {
                        update::<_, u64>(
                            &env.pic,
                            env.observed_ledger,
                            "debug_append_transfers",
                            transfers,
                        )?;
                    }
                    let memory_before = env
                        .pic
                        .canister_status(env.event_horizon, None)
                        .map_err(|e| anyhow!("{e:?}"))?
                        .memory_metrics
                        .wasm_memory_size
                        .0;
                    let dependency_ids = [env.ledger, env.observed_ledger, env.historian];
                    let ledger_before = dependency_ids
                        .into_iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .map(|id| (id, env.pic.cycle_balance(id)))
                        .collect::<Vec<_>>();
                    let recipient_before = receivers
                        .iter()
                        .map(|id| env.pic.cycle_balance(*id))
                        .collect::<Vec<_>>();
                    // Restore identical actual launch liquidity after excluded
                    // seeding, installation and management-status charges.
                    let fixture_credit = initial_cycles
                        .checked_sub(env.pic.cycle_balance(env.event_horizon))
                        .context("fixture unexpectedly gained cycles")?;
                    env.pic.add_cycles(env.event_horizon, fixture_credit);
                    // VeryFast: one due poll after 10s, no hourly maintenance.
                    env.pic
                        .advance_time(std::time::Duration::from_secs(window_seconds));
                    let before = env.pic.cycle_balance(env.event_horizon);
                    assert_eq!(before, initial_cycles);
                    for _ in 0..200 {
                        env.pic.tick();
                    }
                    let after = env.pic.cycle_balance(env.event_horizon);
                    for _ in 0..20 {
                        env.pic.tick();
                    }
                    assert_eq!(
                        after,
                        env.pic.cycle_balance(env.event_horizon),
                        "production cost not settled"
                    );
                    let sender_cycles = before
                        .checked_sub(after)
                        .context("unexpected sender credit")?;
                    let recipient_cycles = receivers
                        .iter()
                        .zip(recipient_before)
                        .map(|(id, b)| {
                            b.checked_sub(env.pic.cycle_balance(*id))
                                .expect("recipient credit")
                        })
                        .sum::<u128>();
                    let dependency_costs = ledger_before
                        .into_iter()
                        .map(|(id, b)| {
                            (
                                id,
                                b.checked_sub(env.pic.cycle_balance(id))
                                    .expect("dependency credit"),
                            )
                        })
                        .collect::<std::collections::BTreeMap<_, _>>();
                    let historian_cycles = dependency_costs[&env.historian];
                    let ledger_cycles = dependency_costs
                        .iter()
                        .filter(|(id, _)| **id != env.historian)
                        .map(|(_, c)| *c)
                        .sum::<u128>();
                    let status = env
                        .pic
                        .canister_status(env.event_horizon, None)
                        .map_err(|e| anyhow!("{e:?}"))?;
                    let mut receipts = Vec::new();
                    let mut payload_bytes = 0usize;
                    for (id, previous) in receivers.iter().zip(&receiver_counts) {
                        let expected_count = if scenario == "real_unmatched_increasing" {
                            *previous
                        } else {
                            previous + 1
                        };
                        assert_eq!(
                            poke_count(&env, *id)?,
                            expected_count,
                            "missing/extra genuine production callback to{id}"
                        );
                        if scenario == "real_unmatched_increasing" {
                            continue;
                        }
                        assert_eq!(
                            query::<_, Option<Principal>>(&env.pic, *id, "debug_last_caller", ())?,
                            Some(env.event_horizon)
                        );
                        let actual = notification(&env, *id)?;
                        let expected = if scenario == "real_global" {
                            PokeNotification {
                                global_activity: true,
                                matches: vec![],
                            }
                        } else if scenario == "real_growing" {
                            PokeNotification {
                                global_activity: false,
                                matches: (0..256)
                                    .map(|n| PokeMatch {
                                        subject: env.subscriber,
                                        target: WatchTarget::Subaccount(n),
                                        first_block_index: initial.observed_next_block + n,
                                        max_amount: candid::Nat::from(if n == 0 {
                                            1000u64
                                        } else {
                                            20u64
                                        }),
                                    })
                                    .collect(),
                            }
                        } else {
                            PokeNotification {
                                global_activity: false,
                                matches: vec![PokeMatch {
                                    subject: env.subscriber,
                                    target: WatchTarget::Subaccount(7),
                                    first_block_index: initial.observed_next_block,
                                    max_amount: candid::Nat::from(
                                        if scenario == "real_increasing" {
                                            blocks
                                        } else {
                                            20
                                        },
                                    ),
                                }],
                            }
                        };
                        assert_eq!(actual, expected, "complete production payload to{id}");
                        payload_bytes += encode_one(&actual)?.len();
                        receipts.push((*id, actual));
                    }
                    let callbacks = if scenario == "real_unmatched_increasing" {
                        0
                    } else {
                        receivers.len()
                    };
                    // Inspect only after cost and all genuine receipts are fixed.
                    env.pic
                        .upgrade_canister(
                            env.event_horizon,
                            seed.clone(),
                            candid::encode_args(())?,
                            None,
                        )
                        .map_err(|e| anyhow!("inspection upgrade{e:?}"))?;
                    let state = env.state()?;
                    assert_eq!(
                        state.observed_next_block,
                        initial.observed_next_block + blocks,
                        "incomplete observed scan"
                    );
                    assert_eq!(
                        state.admission_next_block,
                        initial.admission_next_block + if generic { 0 } else { blocks },
                        "admission stream changed"
                    );
                    assert_eq!(state.subscriptions, initial.subscriptions);
                    assert_eq!(state.global_subscriptions, initial.global_subscriptions);
                    assert_eq!(state.cmc_state, initial.cmc_state);
                    assert_eq!(state.polling_mode, PollingMode::VeryFast);
                    let semantic = encode_one((&state, &receipts))?;
                    if let Some(ref old) = reference {
                        assert_eq!(old, &semantic, "production semantic differential");
                    } else {
                        reference = Some(semantic);
                    }
                    csv.push_str(&format!("{variant},{sample},{generic},{scenario},{blocks},{entries},{sender_cycles},{recipient_cycles},{ledger_cycles},{historian_cycles},{callbacks},{payload_bytes},{},{},{},{},{},{},{canonical},{},{initial_cycles},{window_seconds},VeryFast,{before},{after},{fixture_credit}\n",initial.observed_next_block,state.observed_next_block,state.admission_next_block,status.memory_metrics.stable_memory_size.0,memory_before,status.memory_metrics.wasm_memory_size.0,status.idle_cycles_burned_per_day.0));
                }
            }
        }
    }
    save_complete("EH_OPT_PRODUCTION_OUTPUT", &csv)?;
    print!("{csv}");
    Ok(())
}
