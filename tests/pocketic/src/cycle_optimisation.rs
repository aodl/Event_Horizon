// Deliberately separate installed instances; never mutate or rebuild the baseline.
mod cycle_optimisation {
    use super::*;

    fn baseline() -> Result<Vec<u8>> {
        let path = std::env::var_os("EH_OPT_BASELINE")
            .map(PathBuf::from)
            .unwrap_or(workspace_root()?.join("tests/fixtures/event_horizon_81c5a0d_debug.wasm"));
        let bytes = std::fs::read(&path)
            .with_context(|| format!("baseline artifact {}", path.display()))?;
        if std::env::var_os("EH_OPT_BASELINE").is_none() {
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                "325ea3d83c1694a2ccdd4c30b4473500fc9a4f3326bc1b0d1fc1dbea049b94ff"
            );
        }
        Ok(bytes)
    }

    #[cfg(feature = "optimisation_benchmarks")]
    #[derive(CandidType, Deserialize)]
    struct Metric {
        phase: String,
        calls: u64,
        instructions: u64,
        linear_heap_pages: u64,
    }
    // Select complete pairs for short-lived PocketIC processes. Defaults retain
    // all three independent samples; the runner combines only complete shards.
    fn selected_generic(generic: bool) -> bool {
        std::env::var("EH_OPT_GENERIC").map_or(true, |v| v == generic.to_string())
    }
    fn selected_scenario(name: &str, blocks: u64, watchers: u64) -> bool {
        std::env::var("EH_OPT_CASE").map_or(true, |v| v == format!("{name}:{blocks}:{watchers}"))
    }
    fn samples() -> Vec<usize> {
        if let Ok(sample) = std::env::var("EH_OPT_SAMPLE") {
            let sample: usize = sample.parse().expect("sample index");
            assert!(sample < 3);
            vec![sample]
        } else {
            (0..3).collect()
        }
    }
    fn save_complete(path_variable: &str, csv: &str) -> Result<()> {
        if let Some(path) = std::env::var_os(path_variable) {
            let path = PathBuf::from(path);
            assert!(
                !path.exists(),
                "refuse to overwrite prior evidence: {}",
                path.display()
            );
            std::fs::write(path, csv)?;
        }
        Ok(())
    }

    #[test]
    #[ignore = "serial installed-Wasm baseline/candidate measurements"]
    #[cfg(feature = "optimisation_benchmarks")]
    fn paired_fixed_work() -> Result<()> {
        let instrumented = std::env::var_os("EH_OPT_METRICS_OUTPUT").is_some();
        let mut metrics_csv = String::from(
            "variant,sample,generic,scenario,phase,calls,instructions,linear_heap_pages\n",
        );
        #[derive(CandidType)]
        struct Seed {
            subject: Principal,
            start: u64,
            count: u64,
        }
        let old = baseline()?;
        let new = if let Some(path) = std::env::var_os("EH_OPT_CANDIDATE") {
            std::fs::read(path)?
        } else {
            wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?
        };
        // Warm fixture compilation before measurement. Timed regions contain no builds.
        for (cache, package) in [
            (&LEDGER_WASM, "mock-icp-ledger"),
            (&ICRC3_LEDGER_WASM, "mock-icrc3-ledger"),
            (&HISTORIAN_WASM, "mock-historian"),
            (&CMC_WASM, "mock-cmc"),
            (&SUBSCRIBER_WASM, "mock-subscriber"),
        ] {
            wasm(cache, package, None)?;
        }
        let full = std::env::var_os("EH_OPT_FULL").is_some();
        let scenarios: Vec<(&str, u64, u64)> = if full {
            vec![
                ("idle", 0, 0),
                ("idle", 0, 256),
                ("idle", 0, 1024),
                ("repeat_idle", 0, 0),
                ("repeat_idle", 0, 256),
                ("repeat_idle", 0, 1024),
                ("bootstrap", 1, 0),
                ("bootstrap", 1, 256),
                ("bootstrap", 1, 1024),
                ("archive_only", 512, 0),
                ("archive_only", 512, 256),
                ("archive_only", 512, 1024),
                ("global", 1, 0),
                ("global", 1, 1),
                ("global", 1, 256),
                ("global", 1, 1024),
                ("mixed", 1, 1024),
                ("ordinary", 256, 0),
                ("ordinary", 512, 0),
                ("ordinary", 4096, 0),
                ("ordinary", 16384, 0),
                ("equal", 256, 1),
                ("equal", 256, 16),
                ("equal", 1, 1024),
                ("equal", 32, 1024),
                ("equal", 256, 1024),
                ("increasing", 256, 1024),
                ("decreasing", 256, 1024),
                ("alternating", 256, 1024),
                ("random", 256, 1024),
                ("zero", 256, 1024),
                ("unique_unwatched", 256, 0),
                ("unique_watched", 256, 256),
                ("irrelevant_memo", 256, 0),
                ("wrong_source", 256, 0),
                ("wrong_destination", 256, 0),
                ("nat257", 32, 1024),
                ("nat4097", 32, 1024),
                ("nat32769", 32, 16),
                ("growing", 256, 512),
                ("growing", 512, 512),
                ("growing", 4096, 512),
                ("growing", 16384, 512),
                ("unique", 256, 16),
                ("backlog", 256, 16),
                ("backlog", 512, 16),
                ("backlog", 4096, 16),
                ("backlog", 16384, 16),
            ]
        } else {
            vec![("equal", 32, 16)]
        };
        let mut csv = String::from("variant,sample,generic,scenario,blocks,watchers,sender_cycles,immediate_sender_cycles,stable_bytes,total_bytes,attempts,accepted,cursor,simulated_nanos,legacy_calls,icrc3_calls,ledger_cycles,historian_cycles,cmc_cycles,recipient_cycles,typed_payload_bytes,start_cursor,wasm_heap_before,wasm_heap_after,completed_polls,processed_blocks,fixture_phase,sender_balance_before,sender_balance_after\n");
        for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
            for &(scenario, blocks, watchers) in &scenarios {
                if !selected_scenario(scenario, blocks, watchers)
                    || (scenario.starts_with("nat") && !generic)
                {
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
                        if scenario != "bootstrap" {
                            env.poll()?;
                        }
                        if matches!(
                            scenario,
                            "idle" | "repeat_idle" | "bootstrap" | "archive_only" | "global"
                        ) {
                            env.seed_subscribers("global", watchers)?;
                        } else if scenario == "growing" {
                            env.admit_range(0, 255, None, 20 * ICP_E8S)?;
                            env.admit_range(256, 511, None, 20 * ICP_E8S)?;
                        } else if scenario == "unique_watched" {
                            env.seed_subscribers("specific", watchers)?;
                        } else {
                            let specific_watchers = if scenario == "mixed" {
                                env.seed_subscribers("global", watchers / 2)?;
                                watchers / 2
                            } else {
                                watchers
                            };
                            for start in (0..specific_watchers).step_by(128) {
                                update::<_, ()>(
                                    &env.pic,
                                    env.event_horizon,
                                    "debug_seed_external",
                                    Seed {
                                        subject: env.subscriber,
                                        start,
                                        count: (specific_watchers - start).min(128),
                                    },
                                )?;
                            }
                        }
                        let cursor_before = env.state()?.observed_next_block;
                        let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                        let mut largest = candid::Nat::from(0u8);
                        if scenario == "unique_watched" {
                            env.append_observed_batch(DebugBatchKind::Specific, blocks)?;
                            largest = blocks.into();
                        } else if scenario.starts_with("nat") {
                            let bits: usize = scenario.trim_start_matches("nat").parse()?;
                            largest = candid::Nat(candid::Nat::from(1u8).0 << (bits - 1));
                            for _ in 0..blocks {
                                env.append_observed_nat(
                                    source.clone(),
                                    account_id(env.subscriber, numbered(7)),
                                    largest.clone(),
                                )?;
                            }
                        } else {
                            for start in (0..blocks).step_by(256) {
                                let mut batch = Vec::new();
                                for i in start..(start + 256).min(blocks) {
                                    let amount = match scenario {
                                        "increasing" => i + 1,
                                        "decreasing" => blocks - i,
                                        "random" => {
                                            i.wrapping_mul(6364136223846793005)
                                                .wrapping_add(1442695040888963407)
                                                .rotate_right(32)
                                                % 32
                                        }
                                        "zero" => 0,
                                        "growing" if i + 1 == blocks && blocks > 256 => 1000,
                                        "alternating" => {
                                            if i % 2 == 0 {
                                                1000
                                            } else {
                                                1
                                            }
                                        }
                                        _ => 20,
                                    };
                                    largest = largest.max(candid::Nat::from(amount));
                                    let target = match scenario {
                                        "unique" | "unique_unwatched" => i,
                                        "growing" if i + 1 == blocks && blocks > 256 => 0,
                                        "growing" => i,
                                        _ => 7,
                                    };
                                    let payout = matches!(
                                        scenario,
                                        "wrong_source" | "wrong_destination" | "irrelevant_memo"
                                    );
                                    batch.push(Append {
                                        from: if payout && scenario != "wrong_source" {
                                            account_id(env.faucet, [0; 32])
                                        } else {
                                            source.clone()
                                        },
                                        to: if payout && scenario != "wrong_destination" {
                                            account_id(env.event_horizon, [0; 32])
                                        } else {
                                            account_id(env.subscriber, numbered(target))
                                        },
                                        amount_e8s: amount,
                                        icrc1_memo: if scenario == "irrelevant_memo" {
                                            Some(vec![b'x'; 4096])
                                        } else if payout {
                                            Some(
                                                format!(
                                                    "{}.7",
                                                    env.subscriber.to_text().replace('-', "")
                                                )
                                                .into_bytes(),
                                            )
                                        } else {
                                            None
                                        },
                                    });
                                }
                                update::<_, u64>(
                                    &env.pic,
                                    env.observed_ledger,
                                    "debug_append_transfers",
                                    batch,
                                )?;
                            }
                        }
                        if scenario == "archive_only" {
                            update::<_, ()>(
                                &env.pic,
                                env.observed_ledger,
                                "debug_set_first_local_block",
                                cursor_before + blocks,
                            )?;
                        }
                        // Fixture installation/allocation accounting is not poll
                        // work. Settle it at the original clock before the snapshot.
                        for _ in 0..60 {
                            env.pic.tick();
                        }
                        let setup_settled = env.pic.cycle_balance(env.event_horizon);
                        for _ in 0..10 {
                            env.pic.tick();
                        }
                        assert_eq!(
                            env.pic.cycle_balance(env.event_horizon),
                            setup_settled,
                            "fixture setup accounting not settled"
                        );
                        let count_calls = || -> Result<(u64, u64)> {
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
                        let calls_before = count_calls()?;
                        let dependencies = [
                            env.ledger,
                            env.observed_ledger,
                            env.historian,
                            env.cmc,
                            env.subscriber,
                        ];
                        let dependency_before: Vec<_> = dependencies
                            .iter()
                            .map(|p| env.pic.cycle_balance(*p))
                            .collect();
                        let memory_before = env
                            .pic
                            .canister_status(env.event_horizon, None)
                            .map_err(|e| anyhow!("{e:?}"))?
                            .memory_metrics
                            .wasm_memory_size
                            .0;
                        let expected = if blocks == 0
                            || watchers == 0
                            || matches!(scenario, "bootstrap" | "archive_only")
                        {
                            0
                        } else if scenario == "growing" {
                            1
                        } else if scenario == "zero" {
                            watchers.div_ceil(16)
                        } else {
                            watchers
                        };
                        let completed_polls = if scenario == "repeat_idle" { 8 } else { 1 };
                        if instrumented {
                            update::<_, ()>(
                                &env.pic,
                                env.event_horizon,
                                "debug_reset_optimisation_metrics",
                                (),
                            )?;
                            for _ in 0..10 {
                                env.pic.tick();
                            }
                        }
                        let counts = env.poke_counts()?;
                        let start = env.pic.get_time();
                        let cycles = env.pic.cycle_balance(env.event_horizon);
                        for _ in 0..completed_polls {
                            env.poll()?;
                        }
                        // Fixed settlement rounds at unchanged simulated time. Queries/status are outside cost region.
                        for _ in 0..30 {
                            env.pic.tick();
                        }
                        let immediate = cycles
                            .checked_sub(env.pic.cycle_balance(env.event_horizon))
                            .context("unexpected sender credit")?;
                        // Synthetic recipients can be unroutable. Allow the ordinary
                        // bounded-call timeout before classifying their reservations
                        // as settled consumption. Debug fixtures have no autonomous
                        // scheduler; storage accounting over this interval is included.
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
                            env.pic.cycle_balance(env.event_horizon),
                            settled,
                            "sender has unsettled refunds/execution"
                        );
                        let used = cycles
                            .checked_sub(settled)
                            .context("unexpected settled credit")?;
                        let end = env.pic.get_time();
                        let state = env.state()?;
                        assert_eq!(
                            state.observed_next_block,
                            cursor_before + blocks,
                            "incomplete scan"
                        );
                        let calls_after = count_calls()?;
                        let calls = (
                            calls_after.0 - calls_before.0,
                            calls_after.1 - calls_before.1,
                        );
                        let dependency_used: Vec<_> = dependencies
                            .iter()
                            .zip(&dependency_before)
                            .map(|(p, b)| {
                                b.checked_sub(env.pic.cycle_balance(*p))
                                    .expect("unexpected dependency credit")
                            })
                            .collect();
                        let after = env.poke_counts()?;
                        assert_eq!(
                            after[0] - counts[0],
                            expected,
                            "skipped send is not a saving"
                        );
                        assert_eq!(
                            after[1] - counts[1],
                            expected,
                            "failed send is not a saving"
                        );
                        let observed_notification = if scenario == "growing" {
                            env.wait_for_wakeup_pokes(env.subscriber, 1)?;
                            let n = notification(&env, env.subscriber)?;
                            assert!(!n.global_activity);
                            assert_eq!(n.matches.len(), 256);
                            for (target, m) in n.matches.iter().enumerate() {
                                assert_eq!(m.subject, env.subscriber);
                                assert_eq!(m.target, WatchTarget::Subaccount(target as u64));
                                assert_eq!(m.first_block_index, cursor_before + target as u64);
                                assert_eq!(
                                    m.max_amount,
                                    candid::Nat::from(if target == 0 && blocks > 256 {
                                        1000u64
                                    } else {
                                        20u64
                                    })
                                );
                            }
                            Some(n)
                        } else {
                            None
                        };
                        let payload_bytes = if let Some(ref n) = observed_notification {
                            encode_one(n)?.len() as u64
                        } else if scenario == "mixed" {
                            let global = encode_one(PokeNotification {
                                global_activity: true,
                                matches: vec![],
                            })?
                            .len() as u64;
                            let specific = encode_one(PokeNotification {
                                global_activity: false,
                                matches: vec![PokeMatch {
                                    subject: env.subscriber,
                                    target: WatchTarget::Subaccount(7),
                                    first_block_index: cursor_before,
                                    max_amount: largest.clone(),
                                }],
                            })?
                            .len() as u64;
                            global * (watchers / 2) + specific * (watchers / 2)
                        } else if scenario == "unique_watched" {
                            (0..blocks)
                                .map(|i| {
                                    let mut bytes = [0u8; 9];
                                    bytes[0] = 0x7f;
                                    bytes[1..].copy_from_slice(&i.to_be_bytes());
                                    encode_one(PokeNotification {
                                        global_activity: false,
                                        matches: vec![PokeMatch {
                                            subject: Principal::from_slice(&bytes),
                                            target: WatchTarget::Subaccount(0),
                                            first_block_index: cursor_before + i,
                                            max_amount: (i + 1).into(),
                                        }],
                                    })
                                    .map(|b| b.len() as u64)
                                })
                                .collect::<std::result::Result<Vec<_>, _>>()?
                                .into_iter()
                                .sum()
                        } else {
                            encode_one(PokeNotification {
                                global_activity: scenario == "global",
                                matches: if scenario == "global" {
                                    vec![]
                                } else {
                                    vec![PokeMatch {
                                        subject: env.subscriber,
                                        target: WatchTarget::Subaccount(7),
                                        first_block_index: cursor_before,
                                        max_amount: largest.clone(),
                                    }]
                                },
                            })?
                            .len() as u64
                                * expected
                        };
                        if instrumented {
                            let metrics: Vec<Metric> = query(
                                &env.pic,
                                env.event_horizon,
                                "debug_optimisation_metrics",
                                (),
                            )?;
                            assert!(!metrics.is_empty());
                            for m in metrics {
                                metrics_csv.push_str(&format!(
                                    "{variant},{sample},{generic},{scenario},{},{},{},{}\n",
                                    m.phase, m.calls, m.instructions, m.linear_heap_pages
                                ));
                            }
                        }
                        let semantic = encode_one((&state, calls, observed_notification))?;
                        if let Some(ref old) = reference {
                            assert_eq!(old, &semantic, "state differential {scenario}");
                        } else {
                            reference = Some(semantic);
                        }
                        let status = env
                            .pic
                            .canister_status(env.event_horizon, None)
                            .map_err(|e| anyhow!("{e:?}"))?;
                        let elapsed =
                            end.as_nanos_since_unix_epoch() - start.as_nanos_since_unix_epoch();
                        let ledger_cost =
                            dependency_used[0] + if generic { dependency_used[1] } else { 0 };
                        csv.push_str(&format!("{variant},{sample},{generic},{scenario},{blocks},{watchers},{used},{immediate},{},{},{expected},{expected},{blocks},{elapsed},{},{},{ledger_cost},{},{},{},{payload_bytes},{cursor_before},{},{},{completed_polls},{},{},{cycles},{settled}\n",
                            status.memory_metrics.stable_memory_size.0,status.memory_size.0,calls.0,calls.1,dependency_used[2],dependency_used[3],dependency_used[4],memory_before,status.memory_metrics.wasm_memory_size.0,if matches!(scenario,"bootstrap"|"archive_only") {0}else{blocks},if scenario=="bootstrap" {"first_bootstrap"}else{"bootstrapped"}));
                    }
                }
            }
        }
        if instrumented {
            save_complete("EH_OPT_METRICS_OUTPUT", &metrics_csv)?;
        }
        save_complete("EH_OPT_OUTPUT", &csv)?;
        println!("{csv}");
        Ok(())
    }
    #[test]
    #[ignore = "diagnose sender settlement separately from source savings"]
    #[cfg(feature = "optimisation_benchmarks")]
    fn settlement_timeline() -> Result<()> {
        for kind in ["real", "synthetic_global", "synthetic_external"] {
            let env = Env::with_anchors_at(
                baseline()?,
                2_000_000_000_000_000,
                false,
                false,
                false,
                false,
                Some(pocket_ic::Time::from_nanos_since_unix_epoch(
                    1_700_000_000_000_000_000,
                )),
            )?;
            env.price_once()?;
            env.poll()?;
            if kind == "real" {
                env.admit_global(GLOBAL_BASE_E8S)?;
            } else if kind == "synthetic_global" {
                env.seed_subscribers("global", 1)?;
            } else {
                #[derive(CandidType)]
                struct Seed {
                    subject: Principal,
                    start: u64,
                    count: u64,
                }
                update::<_, ()>(
                    &env.pic,
                    env.event_horizon,
                    "debug_seed_external",
                    Seed {
                        subject: env.subscriber,
                        start: 0,
                        count: 1,
                    },
                )?;
            }
            env.append_observed(
                account_id(Principal::from_slice(&[91, 1]), [0; 32]),
                account_id(env.subscriber, numbered(7)),
                20,
                None,
            )?;
            for _ in 0..70 {
                env.pic.tick();
            }
            let before = env.pic.cycle_balance(env.event_horizon);
            env.poll()?;
            eprintln!(
                "settlement {kind} after_poll={}",
                before - env.pic.cycle_balance(env.event_horizon)
            );
            for n in 1u64..=64 {
                env.pic.tick();
                if n.is_power_of_two() {
                    eprintln!(
                        "settlement {kind} tick={n} used={}",
                        before - env.pic.cycle_balance(env.event_horizon)
                    );
                }
            }
            env.pic.advance_time(std::time::Duration::from_secs(301));
            for _ in 0..50 {
                env.pic.tick();
            }
            eprintln!(
                "settlement {kind} after_timeout={} counts={:?}",
                before - env.pic.cycle_balance(env.event_horizon),
                env.poke_counts()?
            );
            let logs = env
                .pic
                .fetch_canister_logs(env.event_horizon, Principal::anonymous())
                .map_err(|e| anyhow!("{e:?}"))?;
            for log in logs.iter().rev().take(3) {
                eprintln!("settlement {kind} log={log:?}");
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "installed-Wasm cache-generation boundary"]
    fn smaller_and_equal_transfers_after_threshold_reduction() -> Result<()> {
        let env = Env::new()?;
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
        env.poll()?;
        fund_suffix(&env, "A@B.7:1")?;
        fund_suffix(&env, "C@B.7:1")?;
        env.poll()?;
        let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
        let destination = account_id(env.cmc, numbered(7));
        env.append(source.clone(), destination.clone(), 5_000_000, None)?;
        fund_suffix(&env, "A@B.7:0.01")?;
        let smaller = env.append(source.clone(), destination.clone(), 2_000_000, None)?;
        env.poll()?;
        env.wait_for_wakeup_pokes(env.subscriber, 1)?;
        assert_eq!(
            matches_for(&env, env.subscriber)?[0].first_block_index,
            smaller
        );
        assert_eq!(poke_count(&env, env.subscriber_second)?, 0);
        let first = env.append(source.clone(), destination.clone(), 2_000_000, None)?;
        fund_suffix(&env, "C@B.7:0.01")?;
        let equal = env.append(source, destination, 2_000_000, None)?;
        env.poll()?;
        env.wait_for_wakeup_pokes(env.subscriber, 2)?;
        env.wait_for_wakeup_pokes(env.subscriber_second, 1)?;
        assert_eq!(
            matches_for(&env, env.subscriber)?[0].first_block_index,
            first
        );
        assert_eq!(
            matches_for(&env, env.subscriber_second)?[0].first_block_index,
            equal
        );
        Ok(())
    }
    #[test]
    #[ignore = "reviewed 81c5a0d stable-state upgrade and optimised-path preservation"]
    fn reviewed_baseline_upgrade_preserves_relations_aliases_and_pending_funds() -> Result<()> {
        let candidate = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
        for generic in [false, true] {
            for transfer_pending in [false, true] {
                let env = Env::with_event_horizon_wasm_and_surplus_mode(
                    baseline()?,
                    200_000_000_000_000,
                    true,
                    generic,
                    false,
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
                env.admit(7, Some("1"), 1_000_000_000)?;
                env.admit(7, Some("0.01"), 1_000_000_000)?;
                fund_suffix(&env, "A@B.7:0.02")?;
                fund_suffix(&env, "C@B.7:1")?;
                env.poll()?;
                env.admit_global(100_000 * ICP_E8S)?;
                env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
                env.set_surplus_level(19)?;
                env.set_balance(100_000_000)?;
                if transfer_pending {
                    update::<_, ()>(
                        &env.pic,
                        env.ledger,
                        "debug_set_legacy_behavior",
                        LegacyBehavior::CreatedInFuture,
                    )?;
                } else {
                    update::<_, ()>(
                        &env.pic,
                        env.cmc,
                        "debug_set_behavior",
                        CmcBehavior::Processing,
                    )?;
                }
                env.fund()?;
                let snapshot = encode_one(env.state()?)?;
                let pending = env.state()?.cmc_state;
                assert!(pending.contains(if transfer_pending {
                    "CmcTransferPending"
                } else {
                    "NotifyPending"
                }));
                assert!(pending.contains("planned_surplus: Some(PlannedSurplus"));
                let alias: AliasMetadata =
                    query(&env.pic, env.event_horizon, "debug_alias_metadata", ())?;
                let before_alias = encode_one(alias)?;
                let before_self = encode_one(env.subscription(7)?)?;
                let before_external = encode_one(preview(&env, "A@B.7")?)?;
                env.upgrade_with_debug_wasm(candidate.clone())?;
                assert_eq!(encode_one(env.state()?)?, snapshot);
                assert_eq!(
                    encode_one(query::<_, AliasMetadata>(
                        &env.pic,
                        env.event_horizon,
                        "debug_alias_metadata",
                        ()
                    )?)?,
                    before_alias
                );
                assert_eq!(encode_one(env.subscription(7)?)?, before_self);
                assert_eq!(encode_one(preview(&env, "A@B.7")?)?, before_external);
                assert!(env.global_subscription()?);
                // Two unchanged transfers exercise the new page-local cache and
                // unchanged declaration publication after restoration.
                let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                let first = env.append_observed(
                    source.clone(),
                    account_id(env.cmc, numbered(7)),
                    3_000_000,
                    None,
                )?;
                env.append_observed(source, account_id(env.cmc, numbered(7)), 3_000_000, None)?;
                fund_suffix(&env, "A@B.7:0.03")?; // no-op; the lower threshold survives
                let before_pokes = poke_count(&env, env.subscriber)?;
                env.poll()?;
                env.wait_for_wakeup_pokes(env.subscriber, before_pokes + 1)?;
                let n = notification(&env, env.subscriber)?;
                assert!(n.global_activity);
                let m = n
                    .matches
                    .iter()
                    .find(|m| m.subject == env.cmc)
                    .context("external relation lost")?;
                assert_eq!(m.first_block_index, first);
                assert_eq!(m.max_amount, candid::Nat::from(3_000_000u64));
                assert_eq!(env.state()?.cmc_state, pending);
                let exercised = encode_one(env.state()?)?;
                env.upgrade_with_debug_wasm(candidate.clone())?;
                assert_eq!(encode_one(env.state()?)?, exercised);
                assert_eq!(encode_one(env.subscription(7)?)?, before_self);
                assert_eq!(encode_one(preview(&env, "A@B.7")?)?, before_external);
                update::<_, ()>(
                    &env.pic,
                    env.ledger,
                    "debug_set_legacy_behavior",
                    LegacyBehavior::Normal,
                )?;
                update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
                env.fund()?;
                assert!(env.state()?.cmc_state.contains("Idle"));
                assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
            }
        }
        Ok(())
    }

    /// Uninstrumented actual production exports and trust anchors. Debug fixture
    /// setup/inspection is outside the measured production interval.
    #[test]
    #[ignore = "serial fixed-principal production-Wasm comparison"]
    #[cfg(feature = "optimisation_benchmarks")]
    fn production_fixed_work() -> Result<()> {
        let root = workspace_root()?;
        let baseline_override = std::env::var_os("EH_OPT_BASELINE_PROD");
        let baseline = std::fs::read(
            baseline_override
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(root.join("tests/fixtures/event_horizon_81c5a0d_production.wasm")),
        )?;
        if baseline_override.is_none() {
            assert_eq!(
                format!("{:x}", Sha256::digest(&baseline)),
                "2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528"
            );
        }
        let instrumented = std::env::var_os("EH_OPT_METRICS_OUTPUT").is_some();
        if instrumented {
            assert!(baseline_override.is_some());
            assert!(std::env::var_os("EH_OPT_CANDIDATE_PROD").is_some());
            assert!(std::env::var_os("EH_OPT_CANONICAL_SOURCE").is_none());
        }
        let mut metrics_csv = String::from(
            "variant,sample,generic,scenario,phase,calls,instructions,linear_heap_pages\n",
        );
        let canonical = if let Ok(source) = std::env::var("EH_OPT_CANONICAL_SOURCE") {
            let head = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&root)
                .output()?;
            assert_eq!(source, String::from_utf8(head.stdout)?.trim());
            assert!(Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&root)
                .output()?
                .stdout
                .is_empty());
            let path = std::env::var_os("EH_OPT_CANDIDATE_PROD")
                .context("canonical artifact path required")?;
            assert_eq!(
                std::fs::canonicalize(path)?,
                std::fs::canonicalize(root.join("release-artifacts/event_horizon.wasm"))?
            );
            true
        } else {
            false
        };
        let candidate = if let Some(path) = std::env::var_os("EH_OPT_CANDIDATE_PROD") {
            std::fs::read(path)?
        } else {
            wasm(&EVENT_HORIZON_PROD_WASM, "event-horizon", None)?
        };
        let seed = super::cycle_optimisation::baseline()?;
        for (cache, package) in [
            (&LEDGER_WASM, "mock-icp-ledger"),
            (&ICRC3_LEDGER_WASM, "mock-icrc3-ledger"),
            (&HISTORIAN_WASM, "mock-historian"),
            (&CMC_WASM, "mock-cmc"),
            (&SUBSCRIBER_WASM, "mock-subscriber"),
        ] {
            wasm(cache, package, None)?;
        }
        let mut csv=String::from("variant,sample,generic,scenario,blocks,entries,sender_cycles,stable_bytes,total_bytes,received,canonical_candidate,instrumented,wasm_heap_before,wasm_heap_after\n");
        for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
            for (scenario, blocks, entries) in [
                ("ordinary", 256, 0),
                ("global", 1, 1),
                ("idle", 0, 0),
                ("idle", 0, 256),
                ("idle", 0, 1024),
                ("repeat_idle", 0, 0),
                ("repeat_idle", 0, 256),
                ("repeat_idle", 0, 1024),
                ("bootstrap", 1, 0),
                ("bootstrap", 1, 256),
                ("bootstrap", 1, 1024),
                ("archive_only", 512, 0),
                ("archive_only", 512, 256),
                ("archive_only", 512, 1024),
                ("new_range", 1, 256),
                ("specific", 256, 256),
            ] {
                if !selected_scenario(scenario, blocks, entries) {
                    continue;
                }
                for sample in samples() {
                    let mut reference = None;
                    for (variant, bytes) in [("baseline", &baseline), ("candidate", &candidate)] {
                        eprintln!("production phase=setup variant={variant} generic={generic} scenario={scenario} sample={sample}");
                        let env = Env::with_anchors(
                            seed.clone(),
                            6_000_000_000_000,
                            false,
                            generic,
                            false,
                            true,
                        )?;
                        eprintln!("fixture topology={:?}", env.pic.topology());
                        eprintln!("production phase=price");
                        env.price_once()?;
                        eprintln!("production phase=bootstrap");
                        env.poll()?;
                        eprintln!("production phase=aliases");
                        refresh_aliases(&env)?;
                        if scenario == "global" {
                            env.admit_global(GLOBAL_BASE_E8S)?;
                        }
                        if scenario == "idle" {
                            env.seed_subscribers("global", entries)?;
                        }
                        if scenario == "specific" {
                            env.admit_range(0, 255, None, 20 * ICP_E8S)?;
                        }
                        let admission_memo =
                            format!("{}.0-255", env.subscriber.to_text().replace('-', ""))
                                .into_bytes();
                        if scenario == "new_range" {
                            env.set_route(admission_memo.clone(), 20 * ICP_E8S)?;
                        }
                        let initial = env.state()?;
                        eprintln!("production phase=upgrade");
                        env.pic
                            .upgrade_canister(
                                env.event_horizon,
                                bytes.clone(),
                                candid::encode_args(())?,
                                None,
                            )
                            .map_err(|e| anyhow!("production upgrade {e:?}"))?;
                        for _ in 0..40 {
                            env.pic.tick();
                        } // Settle install/upgrade scheduler lanes outside measurement.
                        let settled = env.pic.cycle_balance(env.event_horizon);
                        for _ in 0..10 {
                            env.pic.tick();
                        }
                        assert_eq!(
                            env.pic.cycle_balance(env.event_horizon),
                            settled,
                            "initial production lanes not settled"
                        );
                        let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                        let batch: Vec<_> = (0..blocks)
                            .map(|i| Append {
                                from: if scenario == "new_range" {
                                    account_id(env.faucet, [0; 32])
                                } else {
                                    source.clone()
                                },
                                to: if scenario == "new_range" {
                                    account_id(env.event_horizon, [0; 32])
                                } else {
                                    account_id(
                                        env.subscriber,
                                        numbered(if scenario == "specific" { i } else { 7 }),
                                    )
                                },
                                amount_e8s: 20,
                                icrc1_memo: if scenario == "new_range" {
                                    Some(admission_memo.clone())
                                } else {
                                    None
                                },
                            })
                            .collect();
                        update::<_, u64>(
                            &env.pic,
                            if scenario == "new_range" {
                                env.ledger
                            } else {
                                env.observed_ledger
                            },
                            "debug_append_transfers",
                            batch,
                        )?;
                        if instrumented {
                            update::<_, ()>(
                                &env.pic,
                                env.event_horizon,
                                "debug_reset_optimisation_metrics",
                                (),
                            )?;
                        }
                        let memory_before = env
                            .pic
                            .canister_status(env.event_horizon, None)
                            .map_err(|e| anyhow!("{e:?}"))?
                            .memory_metrics
                            .wasm_memory_size
                            .0;
                        // Standard's next poll is due after 600 s. No hourly funding or daily pricing is due.
                        env.pic.advance_time(std::time::Duration::from_secs(600));
                        let before = env.pic.cycle_balance(env.event_horizon);
                        for _ in 0..40 {
                            env.pic.tick();
                        }
                        let settled = env.pic.cycle_balance(env.event_horizon);
                        for _ in 0..10 {
                            env.pic.tick();
                        }
                        assert_eq!(
                            env.pic.cycle_balance(env.event_horizon),
                            settled,
                            "production poll not settled"
                        );
                        let used = before
                            .checked_sub(env.pic.cycle_balance(env.event_horizon))
                            .context("unexpected production credit")?;
                        let status = env
                            .pic
                            .canister_status(env.event_horizon, None)
                            .map_err(|e| anyhow!("{e:?}"))?;
                        let received = poke_count(&env, env.subscriber)?;
                        assert_eq!(
                            received,
                            u64::from(matches!(scenario, "global" | "specific")),
                            "missed or duplicated production wake"
                        );
                        let received_notification = if matches!(scenario, "global" | "specific") {
                            assert_eq!(
                                query::<_, Option<Principal>>(
                                    &env.pic,
                                    env.subscriber,
                                    "debug_last_caller",
                                    ()
                                )?,
                                Some(env.event_horizon)
                            );
                            let n = notification(&env, env.subscriber)?;
                            if scenario == "global" {
                                assert!(n.global_activity && n.matches.is_empty());
                            } else {
                                assert!(!n.global_activity);
                                assert_eq!(n.matches.len(), 256);
                                for (i, m) in n.matches.iter().enumerate() {
                                    assert_eq!(m.subject, env.subscriber);
                                    assert_eq!(m.target, WatchTarget::Subaccount(i as u64));
                                    assert_eq!(
                                        m.first_block_index,
                                        initial.observed_next_block + i as u64
                                    );
                                    assert_eq!(m.max_amount, candid::Nat::from(20u8));
                                }
                            }
                            Some(n)
                        } else {
                            None
                        };
                        if instrumented {
                            let metrics: Vec<Metric> = query(
                                &env.pic,
                                env.event_horizon,
                                "debug_optimisation_metrics",
                                (),
                            )?;
                            assert!(!metrics.is_empty());
                            for m in metrics {
                                metrics_csv.push_str(&format!(
                                    "{variant},{sample},{generic},{scenario},{},{},{},{}\n",
                                    m.phase, m.calls, m.instructions, m.linear_heap_pages
                                ));
                            }
                        }
                        // Stop production execution by argument-free upgrade only after measurement;
                        // the verified baseline debug decoder inspects the same stable encodings.
                        env.pic
                            .upgrade_canister(
                                env.event_horizon,
                                seed.clone(),
                                candid::encode_args(())?,
                                None,
                            )
                            .map_err(|e| anyhow!("inspection upgrade {e:?}"))?;
                        let state = env.state()?;
                        assert_eq!(
                            state.observed_next_block,
                            initial.observed_next_block
                                + if generic && scenario == "new_range" {
                                    0
                                } else {
                                    blocks
                                },
                            "incomplete production scan"
                        );
                        assert_eq!(
                            state.admission_next_block,
                            initial.admission_next_block
                                + if generic && scenario != "new_range" {
                                    0
                                } else {
                                    blocks
                                }
                        );
                        assert_eq!(state.cmc_state, initial.cmc_state);
                        assert_eq!(state.polling_mode, PollingMode::Standard);
                        if scenario == "new_range" {
                            assert_eq!(state.subscriptions, initial.subscriptions + 256);
                        }
                        let semantic = encode_one((&state, received_notification))?;
                        if let Some(ref old) = reference {
                            assert_eq!(old, &semantic, "production state differential");
                        } else {
                            reference = Some(semantic);
                        }
                        csv.push_str(&format!("{variant},{sample},{generic},{scenario},{blocks},{entries},{used},{},{},{received},{canonical},{instrumented},{},{}\n",status.memory_metrics.stable_memory_size.0,status.memory_size.0,memory_before,status.memory_metrics.wasm_memory_size.0));
                    }
                }
            }
        }
        save_complete("EH_OPT_PRODUCTION_OUTPUT", &csv)?;
        if instrumented {
            save_complete("EH_OPT_METRICS_OUTPUT", &metrics_csv)?;
        }
        println!("{csv}");
        Ok(())
    }
    include!("cycle_optimisation_interleaving.rs");
    include!("cycle_optimisation_publication.rs");
    include!("cycle_optimisation_small.rs");
    include!("cycle_optimisation_scheduler.rs");
    include!("cycle_optimisation_frontend.rs");
    include!("cycle_optimisation_reserve.rs");
    include!("cycle_optimisation_production_upgrade.rs");
}
