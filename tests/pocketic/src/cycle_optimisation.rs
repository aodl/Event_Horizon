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
                ("global", 1, 1),
                ("global", 1, 256),
                ("global", 1, 1024),
                ("ordinary", 256, 0),
                ("equal", 1, 1024),
                ("equal", 32, 1024),
                ("equal", 256, 1024),
                ("increasing", 256, 1024),
                ("decreasing", 256, 1024),
                ("alternating", 256, 1024),
                ("unique", 256, 16),
                ("backlog", 256, 16),
                ("backlog", 512, 16),
                ("backlog", 4096, 16),
                ("backlog", 16384, 16),
            ]
        } else {
            vec![("equal", 32, 16)]
        };
        let mut csv = String::from("variant,sample,generic,scenario,blocks,watchers,sender_cycles,stable_bytes,total_bytes,attempts,accepted,cursor,simulated_nanos\n");
        for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
            for &(scenario, blocks, watchers) in &scenarios {
                if !selected_scenario(scenario, blocks, watchers) {
                    continue;
                }
                for sample in samples() {
                    let mut reference = None;
                    for (variant, bytes) in [("baseline", &old), ("candidate", &new)] {
                        let env = Env::with_event_horizon_wasm_and_surplus_mode(
                            bytes.clone(),
                            2_000_000_000_000_000,
                            false,
                            generic,
                            false,
                        )?;
                        env.pic.set_time(
                            (std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000))
                                .into(),
                        );
                        env.price_once()?;
                        env.poll()?;
                        if scenario == "idle" || scenario == "global" {
                            env.seed_subscribers("global", watchers)?;
                        } else {
                            for start in (0..watchers).step_by(128) {
                                update::<_, ()>(
                                    &env.pic,
                                    env.event_horizon,
                                    "debug_seed_external",
                                    Seed {
                                        subject: env.subscriber,
                                        start,
                                        count: (watchers - start).min(128),
                                    },
                                )?;
                            }
                        }
                        let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
                        for start in (0..blocks).step_by(256) {
                            let mut batch = Vec::new();
                            for i in start..(start + 256).min(blocks) {
                                let amount = match scenario {
                                    "increasing" => i + 1,
                                    "decreasing" => blocks - i,
                                    "alternating" => {
                                        if i % 2 == 0 {
                                            1000
                                        } else {
                                            1
                                        }
                                    }
                                    _ => 20,
                                };
                                let target = if scenario == "unique" { i } else { 7 };
                                batch.push(Append {
                                    from: source.clone(),
                                    to: account_id(env.subscriber, numbered(target)),
                                    amount_e8s: amount,
                                    icrc1_memo: None,
                                });
                            }
                            update::<_, u64>(
                                &env.pic,
                                env.observed_ledger,
                                "debug_append_transfers",
                                batch,
                            )?;
                        }
                        let counts = env.poke_counts()?;
                        let start = env.pic.get_time();
                        let cycles = env.pic.cycle_balance(env.event_horizon);
                        env.poll()?;
                        // Fixed settlement rounds at unchanged simulated time. Queries/status are outside cost region.
                        for _ in 0..30 {
                            env.pic.tick();
                        }
                        let used = cycles
                            .checked_sub(env.pic.cycle_balance(env.event_horizon))
                            .context("unexpected sender credit")?;
                        let end = env.pic.get_time();
                        let state = env.state()?;
                        assert_eq!(state.observed_next_block, blocks, "incomplete scan");
                        let after = env.poke_counts()?;
                        let expected = if blocks == 0 || watchers == 0 {
                            0
                        } else {
                            watchers
                        };
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
                        let semantic = encode_one(&state)?;
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
                        csv.push_str(&format!("{variant},{sample},{generic},{scenario},{blocks},{watchers},{used},{},{},{expected},{expected},{blocks},{}\n",status.memory_metrics.stable_memory_size,status.memory_size,elapsed));
                    }
                }
            }
        }
        save_complete("EH_OPT_OUTPUT", &csv)?;
        println!("{csv}");
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
        #[derive(CandidType, Deserialize)]
        struct Metric {
            phase: String,
            calls: u64,
            instructions: u64,
            linear_heap_pages: u64,
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
        let mut csv=String::from("variant,sample,generic,scenario,blocks,sender_cycles,stable_bytes,total_bytes,received,canonical_candidate,instrumented\n");
        for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
            for (scenario, blocks) in [("ordinary", 256), ("global", 1)] {
                if !selected_scenario(scenario, blocks, u64::from(scenario == "global")) {
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
                        eprintln!("production phase=price");
                        env.price_once()?;
                        eprintln!("production phase=bootstrap");
                        env.poll()?;
                        eprintln!("production phase=aliases");
                        refresh_aliases(&env)?;
                        if scenario == "global" {
                            env.admit_global(GLOBAL_BASE_E8S)?;
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
                            .map(|_| Append {
                                from: source.clone(),
                                to: account_id(env.subscriber, numbered(7)),
                                amount_e8s: 20,
                                icrc1_memo: None,
                            })
                            .collect();
                        update::<_, u64>(
                            &env.pic,
                            env.observed_ledger,
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
                            u64::from(scenario == "global"),
                            "missed or duplicated production wake"
                        );
                        if scenario == "global" {
                            let n = notification(&env, env.subscriber)?;
                            assert!(n.global_activity);
                            assert!(n.matches.is_empty());
                        }
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
                            initial.observed_next_block + blocks,
                            "incomplete production scan"
                        );
                        assert_eq!(
                            state.admission_next_block,
                            initial.admission_next_block + if generic { 0 } else { blocks }
                        );
                        assert_eq!(state.cmc_state, initial.cmc_state);
                        assert_eq!(state.polling_mode, PollingMode::Standard);
                        let semantic = encode_one(&state)?;
                        if let Some(ref old) = reference {
                            assert_eq!(old, &semantic, "production state differential");
                        } else {
                            reference = Some(semantic);
                        }
                        csv.push_str(&format!("{variant},{sample},{generic},{scenario},{blocks},{used},{},{},{received},{canonical},{instrumented}\n",status.memory_metrics.stable_memory_size,status.memory_size));
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
}
