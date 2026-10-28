// Fixed-time evaluation of the existing scheduler, distinct from fixed-work
// savings. One PocketIC round per simulated second is an explicit fixture
// latency assumption. No mainnet throughput or runway guarantee is implied.
#[test]
#[ignore = "serial paired unchanged-scheduler fixed-time evaluation"]
#[cfg(feature = "optimisation_benchmarks")]
fn scheduler_fixed_time() -> Result<()> {
    let old = baseline()?;
    let new = std::fs::read(std::env::var_os("EH_OPT_CANDIDATE").context("candidate required")?)?;
    let mut csv=String::from("variant,sample,generic,scenario,seconds,globals,step_seconds,initial_cycles,window_debit,settled_window_plus_drain_cycles,completed_polls,inflight_at_end,extra_drain_polls,observed_blocks,callbacks,max_lag,ending_lag,reserve_seconds,economy_seconds,standard_seconds,fast_seconds,very_fast_seconds,continuous_seconds,idle_cycles_per_day,funding_runs,pricing_runs,alias_success_before,alias_success_after\n");
    for generic in [false, true].into_iter().filter(|g| selected_generic(*g)) {
        for (name, balance, expected_mode, horizon, step_seconds) in [
            (
                "timed_standard",
                6_000_000_000_000u128,
                PollingMode::Standard,
                660u64,
                1u64,
            ),
            (
                "timed_continuous",
                110_000_000_000_000,
                PollingMode::Continuous,
                660,
                1,
            ),
            (
                "timed_daily_standard",
                6_000_000_000_000,
                PollingMode::Standard,
                86_400,
                600,
            ),
        ] {
            for globals in [1u64, 1024] {
                if !selected_scenario(name, horizon, globals) {
                    continue;
                }
                for sample in samples() {
                    for (variant, bytes) in [("baseline", &old), ("candidate", &new)] {
                        let env = Env::with_anchors_at(
                            bytes.clone(),
                            balance,
                            false,
                            generic,
                            false,
                            false,
                            Some(pocket_ic::Time::from_nanos_since_unix_epoch(
                                1_700_000_000_000_000_000,
                            )),
                        )?;
                        eprintln!("fixture topology={:?}", env.pic.topology());
                        env.pic
                            .update_canister_settings(
                                env.event_horizon,
                                None,
                                pocket_ic::CanisterSettings {
                                    freezing_threshold: Some(0u8.into()),
                                    ..Default::default()
                                },
                            )
                            .map_err(|e| anyhow!("{e:?}"))?;
                        env.price_once()?;
                        env.poll()?;
                        refresh_aliases(&env)?;
                        if globals == 1 {
                            env.admit_global(GLOBAL_BASE_E8S)?;
                            env.admit(7, Some("0.01"), 10 * ICP_E8S)?;
                        } else {
                            env.seed_subscribers("global", globals)?;
                        }
                        for _ in 0..70 {
                            env.pic.tick();
                        }
                        update::<_, ()>(&env.pic, env.event_horizon, "debug_start_schedulers", ())?;
                        let mut warm = false;
                        for _ in 0..100 {
                            env.pic.tick();
                            let lanes = ["poll", "funding", "pricing"];
                            if lanes.iter().all(|lane| {
                                env.scheduler_lane_counts(lane).is_ok_and(|n| n[0] > 0)
                                    && env.scheduler_lane_state(lane).is_ok_and(|s| !s[0])
                            }) {
                                warm = true;
                                break;
                            }
                        }
                        assert!(
                            warm,
                            "scheduler startup lanes did not reach a completed boundary"
                        );
                        let starts = env.scheduler_lane_counts("poll")?[0];
                        let funding_starts = env.scheduler_lane_counts("funding")?[0];
                        let pricing_starts = env.scheduler_lane_counts("pricing")?[0];
                        let alias_before: AliasMetadata =
                            query(&env.pic, env.event_horizon, "debug_alias_metadata", ())?;
                        let initial_state = env.state()?;
                        let receiver_before = poke_count(&env, env.subscriber)?;
                        assert!(initial_state.cmc_state.contains("Idle"));
                        assert_eq!(initial_state.polling_mode, expected_mode);
                        let remaining = env.pic.cycle_balance(env.event_horizon);
                        assert!(remaining < balance);
                        env.pic.add_cycles(env.event_horizon, balance - remaining);
                        assert_eq!(env.pic.cycle_balance(env.event_horizon), balance);
                        // The final source fixture clock is identical and monotonic.
                        let start_seconds = 1_700_000_001u64;
                        env.pic
                            .set_time(pocket_ic::Time::from_nanos_since_unix_epoch(
                                start_seconds * 1_000_000_000,
                            ));
                        let before = env.pic.cycle_balance(env.event_horizon);
                        let mut injected = 0u64;
                        let mut maximum_lag = 0u64;
                        let mut tiers = [0u64; 6];
                        for step in 1..=horizon / step_seconds {
                            let second = step * step_seconds;
                            env.pic
                                .advance_time(std::time::Duration::from_secs(step_seconds));
                            let arrival = if horizon == 660 {
                                [5u64, 150, 300, 450].contains(&second)
                            } else {
                                [1u64, 30, 60, 90].contains(&step)
                            };
                            if globals == 1 && arrival {
                                env.append_observed(
                                    account_id(Principal::from_slice(&[91, 1]), [0; 32]),
                                    account_id(env.subscriber, numbered(7)),
                                    2_000_000,
                                    None,
                                )?;
                                injected += 1;
                            }
                            for _ in 0..if step_seconds == 1 { 1 } else { 20 } {
                                env.pic.tick();
                            }
                            let state = env.state()?;
                            let observed =
                                state.observed_next_block - initial_state.observed_next_block;
                            maximum_lag = maximum_lag.max(injected.saturating_sub(observed));
                            let index = match state.polling_mode {
                                PollingMode::ReserveProtection => 0,
                                PollingMode::Economy => 1,
                                PollingMode::Standard => 2,
                                PollingMode::Fast => 3,
                                PollingMode::VeryFast => 4,
                                PollingMode::Continuous => 5,
                            };
                            tiers[index] += step_seconds;
                        }
                        let window_balance = env.pic.cycle_balance(env.event_horizon);
                        let starts_at_end = env.scheduler_lane_counts("poll")?[0];
                        let inflight = u64::from(env.scheduler_lane_state("poll")?[0]);
                        let completed = starts_at_end - starts - inflight;
                        let end_state = env.state()?;
                        let observed =
                            end_state.observed_next_block - initial_state.observed_next_block;
                        assert_eq!(
                            end_state.polling_mode, expected_mode,
                            "ample fixed funding changed tier unexpectedly"
                        );
                        assert_eq!(
                            observed, injected,
                            "traffic did not complete within fixed horizon"
                        );
                        let callbacks = poke_count(&env, env.subscriber)? - receiver_before;
                        if globals == 1 {
                            assert_eq!(
                                callbacks,
                                if expected_mode == PollingMode::Standard && horizon == 660 {
                                    1
                                } else {
                                    4
                                }
                            );
                        } else {
                            assert_eq!(callbacks, 0);
                        }
                        assert!(completed > 0);
                        assert_eq!(tiers.iter().sum::<u64>(), horizon);
                        // After the window only, stop new ordinary work through an
                        // existing debug control and settle reservations. Report all
                        // extra completed polls/control cost; never hide the tail in
                        // a fixed-work saving or call raw debit settled consumption.
                        update::<_, ()>(
                            &env.pic,
                            env.event_horizon,
                            "debug_set_scheduler_liquid_cycles_override",
                            Some(0u128),
                        )?;
                        for _ in 0..200 {
                            env.pic.tick();
                        }
                        assert!(!env.scheduler_lane_state("poll")?[0]);
                        let after = env.pic.cycle_balance(env.event_horizon);
                        for _ in 0..20 {
                            env.pic.tick();
                        }
                        assert_eq!(
                            after,
                            env.pic.cycle_balance(env.event_horizon),
                            "drain did not settle"
                        );
                        let total_completed = env.scheduler_lane_counts("poll")?[0] - starts;
                        let status = env
                            .pic
                            .canister_status(env.event_horizon, None)
                            .map_err(|e| anyhow!("{e:?}"))?;
                        let funding_runs =
                            env.scheduler_lane_counts("funding")?[0] - funding_starts;
                        let pricing_runs =
                            env.scheduler_lane_counts("pricing")?[0] - pricing_starts;
                        let alias_after: AliasMetadata =
                            query(&env.pic, env.event_horizon, "debug_alias_metadata", ())?;
                        if horizon == 86_400 {
                            assert_eq!(completed, 144, "daily Standard polls must all complete");
                            assert_eq!(funding_runs, 24);
                            assert_eq!(pricing_runs, 1);
                            assert!(
                                alias_after.last_success > alias_before.last_success,
                                "daily aliases not refreshed"
                            );
                        }
                        let debit = before
                            .checked_sub(window_balance)
                            .context("window sender credit")?;
                        let settled = before.checked_sub(after).context("settled sender credit")?;
                        csv.push_str(&format!("{variant},{sample},{generic},{name},{horizon},{globals},{step_seconds},{balance},{debit},{settled},{completed},{inflight},{},{observed},{callbacks},{maximum_lag},{},{},{},{},{},{},{},{},{funding_runs},{pricing_runs},{},{}\n",total_completed-completed,injected-observed,tiers[0],tiers[1],tiers[2],tiers[3],tiers[4],tiers[5],status.idle_cycles_burned_per_day.0,alias_before.last_success.map_or_else(String::new,|x|x.to_string()),alias_after.last_success.map_or_else(String::new,|x|x.to_string())));
                    }
                }
            }
        }
    }
    save_complete("EH_OPT_OUTPUT", &csv)?;
    println!("{csv}");
    Ok(())
}
