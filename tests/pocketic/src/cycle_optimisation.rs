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

    #[test]
    #[ignore = "serial installed-Wasm baseline/candidate measurements"]
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
        for generic in [false, true] {
            for &(scenario, blocks, watchers) in &scenarios {
                for sample in 0..3 {
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
                        for i in 0..blocks {
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
                            env.append_observed(
                                source.clone(),
                                account_id(env.subscriber, numbered(target)),
                                amount,
                                None,
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
        if let Some(path) = std::env::var_os("EH_OPT_OUTPUT") {
            std::fs::write(path, &csv)?;
        }
        println!("{csv}");
        Ok(())
    }
}
