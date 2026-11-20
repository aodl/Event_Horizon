fn worker_poke(pic: &PocketIc, backend: Principal, caller: Principal) -> Result<()> {
    let reply = pic
        .update_call(backend, caller, "poke", candid::encode_args(())?)
        .map_err(|error| anyhow!("worker poke: {error}"))?;
    Ok(decode_args(&reply)?)
}

fn wait_for_poke_test(
    pic: &PocketIc,
    description: &str,
    mut ready: impl FnMut() -> Result<bool>,
) -> Result<()> {
    for _ in 0..60 {
        if ready()? {
            return Ok(());
        }
        pic.tick();
    }
    bail!("did not observe {description}")
}

#[derive(CandidType)]
struct WorkerInitArgs {
    observed_ledger: Principal,
    allowed_poke_callers: Option<Vec<Principal>>,
}

#[test]
#[ignore = "runs a pinned historical production Wasm and the candidate in PocketIC"]
fn production_poke_permissions_are_instance_local_and_upgradable() -> Result<()> {
    let candidate = wasm(&EVENT_HORIZON_PROD_WASM, "event-horizon", None)?;
    let old = std::fs::read(
        workspace_root()?.join("tests/fixtures/event_horizon_81c5a0d_production.wasm"),
    )?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&old)),
        "2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528"
    );
    let env = Env::with_anchors(
        wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?,
        8_000_000_000_000,
        false,
        false,
        false,
        true,
    )?;
    let pic = &env.pic;
    let worker = Principal::self_authenticating([1]);
    let other = Principal::self_authenticating([2]);
    let old_arguments = encode_one(InitArgs {
        observed_ledger: env.ledger,
        sns_root: None,
        surplus_canister: None,
    })?;
    let install = |module, args| {
        let id = pic.create_canister();
        pic.add_cycles(id, 8_000_000_000_000);
        pic.install_canister(id, module, args, None);
        id
    };
    let canonical = install(old, old_arguments.clone());
    let upgrade = |argument| {
        pic.upgrade_canister(canonical, candidate.clone(), argument, None)
            .map_err(|error| anyhow!("worker policy upgrade: {error}"))
    };
    let assert_rejected = |id, caller| {
        let reject = pic
            .update_call(id, caller, "poke", candid::encode_args(()).unwrap())
            .expect_err("ingress must be rejected");
        assert_eq!(
            reject.error_code,
            pocket_ic::ErrorCode::CanisterRejectedMessage,
            "{reject}"
        );
    };
    let explicitly_empty = install(
        candidate.clone(),
        encode_one(WorkerInitArgs {
            observed_ledger: env.ledger,
            allowed_poke_callers: Some(vec![]),
        })?,
    );
    let configured = install(
        candidate.clone(),
        encode_one(WorkerInitArgs {
            observed_ledger: env.ledger,
            allowed_poke_callers: Some(vec![other]),
        })?,
    );
    let omitted = install(candidate.clone(), old_arguments);
    upgrade(candid::encode_args(())?)?;
    for id in [canonical, explicitly_empty, omitted] {
        assert_rejected(id, worker);
        assert_rejected(id, Principal::anonymous());
        assert_eq!(
            query::<_, InstanceInfo>(pic, id, "get_instance", ())?.observed_ledger,
            env.ledger
        );
    }
    worker_poke(pic, configured, other)?;
    assert_rejected(configured, worker);

    upgrade(encode_one(Some(vec![worker, env.subscriber]))?)?;
    worker_poke(pic, canonical, worker)?;
    worker_poke(pic, canonical, env.subscriber)?;
    assert_rejected(canonical, other);
    assert_rejected(canonical, Principal::anonymous());
    let forward = |from| {
        update::<_, std::result::Result<(), String>>(pic, from, "debug_forward_poke", canonical)
    };
    assert_eq!(forward(env.subscriber)?, Ok(()));
    assert!(forward(env.subscriber_second)?
        .unwrap_err()
        .contains("unauthorised poke"));

    for args in [
        vec![],
        candid::encode_args(())?,
        encode_one(None::<Vec<Principal>>)?,
    ] {
        upgrade(args)?;
        worker_poke(pic, canonical, worker)?;
    }
    for invalid in [
        encode_one(Some(vec![Principal::anonymous()]))?,
        encode_one(Some(vec![Principal::management_canister()]))?,
        encode_one(Some(vec![worker, worker]))?,
        encode_one(Some(vec![worker; 17]))?,
        encode_one(Some(vec!["not a principal"]))?,
        b"malformed candid".to_vec(),
    ] {
        assert!(
            upgrade(invalid).is_err(),
            "invalid upgrade unexpectedly succeeded"
        );
        worker_poke(pic, canonical, worker)?;
    }
    upgrade(encode_one(Some(vec![other]))?)?;
    assert_rejected(canonical, worker);
    worker_poke(pic, canonical, other)?;
    upgrade(encode_one(Some(Vec::<Principal>::new()))?)?;
    assert_rejected(canonical, other);
    worker_poke(pic, configured, other)?;

    let expected = Sha256::digest(&candidate).to_vec();
    for id in [canonical, configured, explicitly_empty, omitted] {
        assert_eq!(
            pic.canister_status(id, None)
                .map_err(|error| anyhow!("{error}"))?
                .module_hash,
            Some(expected.clone())
        );
    }
    Ok(())
}

fn canonical_worker_environment() -> Result<(Env, [Principal; 2])> {
    let backend = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
    wasm(&HISTORIAN_WASM, "mock-historian", Some("interleaving_gate"))?;
    let env = Env::new()?;
    let callers = [
        Principal::self_authenticating([3]),
        Principal::self_authenticating([4]),
    ];
    env.pic
        .upgrade_canister(
            env.event_horizon,
            backend,
            encode_one(Some(callers.to_vec()))?,
            None,
        )
        .map_err(|error| anyhow!("test policy upgrade: {error}"))?;
    env.set_scheduler_liquid_cycles_override(Some(2_000_000_000_000))?;
    env.poll()?;
    Ok((env, callers))
}

#[test]
#[ignore = "checks default-disabled ingress and autonomous scheduling in PocketIC"]
fn canonical_disabled_poke_leaves_autonomous_poll_intact() -> Result<()> {
    use std::time::Duration;
    let env = Env::new()?;
    let pic = &env.pic;
    let backend = env.event_horizon;
    env.set_scheduler_liquid_cycles_override(Some(2_000_000_000_000))?;
    update::<_, ()>(pic, backend, "debug_start_schedulers", ())?;
    wait_for_poke_test(pic, "prospective ordinary poll", || {
        Ok(
            env.state()?.observed_bootstrapped
                && env.scheduler_lane_state("poll")? == [false, true],
        )
    })?;
    let before = env.state()?.observed_next_block;
    let late = env.append_legacy_other("mint")?;
    assert_eq!(before, late);
    for caller in [Principal::anonymous(), Principal::self_authenticating([1])] {
        assert!(worker_poke(pic, backend, caller).is_err());
    }
    pic.advance_time(Duration::from_secs(3_599));
    pic.tick();
    assert_eq!(env.scheduler_lane_counts("poll")?, [1, 0]);
    assert_eq!(env.state()?.observed_next_block, before);
    pic.advance_time(Duration::from_secs(2));
    wait_for_poke_test(pic, "ordinary Economy reconciliation", || {
        Ok(env.state()?.observed_next_block == late + 1)
    })?;
    assert_eq!(env.scheduler_lane_counts("poll")?, [2, 0]);
    Ok(())
}

#[derive(CandidType, Deserialize)]
struct PokeHistorianGate {
    entered: u64,
    waiting: bool,
}

fn poke_historian_gate(env: &Env) -> Result<PokeHistorianGate> {
    query(&env.pic, env.historian, "debug_route_gate_state", ())
}

fn set_poke_historian_gate(env: &Env, hold: bool) -> Result<()> {
    update(&env.pic, env.historian, "debug_route_gate", hold)
}

#[test]
#[ignore = "holds an actual scheduled poll beyond its pinned ICP boundary"]
fn canonical_worker_pokes_follow_the_pinned_boundary_without_overlapping() -> Result<()> {
    use std::time::Duration;
    for start_with_poke in [false, true] {
        let (env, callers) = canonical_worker_environment()?;
        let pic = &env.pic;
        let backend = env.event_horizon;
        env.admit(7, None, 10 * ICP_E8S)?;
        update::<_, ()>(pic, backend, "debug_start_schedulers", ())?;
        wait_for_poke_test(pic, "initial Economy scheduler pass", || {
            Ok(env.scheduler_lane_counts("poll")?[0] == 1
                && env.scheduler_lane_state("poll")? == [false, true]
                && env.scheduler_lane_counts("funding")?[0] == 1
                && env.scheduler_lane_state("funding")? == [false, true]
                && env.scheduler_lane_counts("pricing")?[0] == 1
                && env.scheduler_lane_state("pricing")? == [false, true])
        })?;
        assert_eq!(env.state()?.polling_mode, PollingMode::Economy);
        let first = env.state()?.observed_next_block;
        let transfer = |target| {
            env.append(
                account_id(env.subscriber_second, [0; 32]),
                account_id(env.subscriber, numbered(target)),
                10,
                None,
            )
        };
        transfer(7)?;
        env.append_account_admission_for(env.subscriber, 8, None, 10 * ICP_E8S)?;
        set_poke_historian_gate(&env, true)?;
        let before_gate = poke_historian_gate(&env)?.entered;

        if start_with_poke {
            pic.advance_time(Duration::from_secs(3_590));
            let queued = [callers[0], callers[1], callers[0]].map(|caller| {
                pic.submit_call(backend, caller, "poke", candid::encode_args(()).unwrap())
            });
            for message in queued {
                let message = message.map_err(|error| anyhow!("{error}"))?;
                let reply = pic
                    .await_call(message)
                    .map_err(|error| anyhow!("{error}"))?;
                decode_args::<()>(&reply)?;
            }
        } else {
            pic.advance_time(Duration::from_secs(3_601));
        }

        wait_for_poke_test(
            pic,
            "Historian wait after capture of the first poll tip",
            || {
                let gate = poke_historian_gate(&env)?;
                Ok(gate.entered == before_gate + 1 && gate.waiting)
            },
        )?;
        assert_eq!(poke_historian_gate(&env)?.entered, before_gate + 1);
        assert_eq!(env.state()?.observed_next_block, first);
        assert_eq!(env.scheduler_lane_state("poll")?, [true, false]);
        let late = first + 2;
        transfer(7)?;
        for worker in [callers[1], callers[0], callers[1]] {
            worker_poke(pic, backend, worker)?;
        }

        if start_with_poke {
            // Cross the superseded Economy deadline with a pending wake and
            // let the ordinary funding lane request rescheduling.
            let minted_before: u64 = query(pic, env.cmc, "debug_calls", ())?;
            env.set_balance(ICP_E8S)?;
            pic.advance_time(Duration::from_secs(11));
            wait_for_poke_test(pic, "funding completion while the wake is pending", || {
                Ok(
                    query::<_, u64>(pic, env.cmc, "debug_calls", ())? == minted_before + 1
                        && env.scheduler_lane_state("funding")? == [false, true],
                )
            })?;
        }
        let caught_up = late + 1 + u64::from(start_with_poke);
        assert!(env.scheduler_lane_counts("funding")?[0] >= 1);
        assert!(env.scheduler_lane_counts("pricing")?[0] >= 1);
        assert_eq!(env.scheduler_lane_counts("poll")?, [2, 0]);
        assert_eq!(env.state()?.observed_next_block, first);
        assert_eq!(env.subscriber_observation()?.pokes, 0);

        set_poke_historian_gate(&env, false)?;
        env.wait_for_subscriber(
            "second notification after one immediate follow-up",
            |observation| observation.pokes == 2,
        )?;
        let observation = env.subscriber_observation()?;
        assert_eq!(observation.last_caller, Some(backend));
        assert_eq!(observation.matches.len(), 1);
        assert_eq!(observation.matches[0].target, WatchTarget::Subaccount(7));
        assert_eq!(observation.matches[0].first_block_index, late);
        assert_eq!(env.state()?.observed_next_block, caught_up);
        assert_eq!(env.state()?.admission_next_block, caught_up);
        assert_eq!(env.scheduler_lane_counts("poll")?, [3, 0]);

        wait_for_poke_test(pic, "restored ordinary timer", || {
            Ok(env.scheduler_lane_state("poll")? == [false, true])
        })?;
        transfer(99)?;
        pic.advance_time(Duration::from_secs(3_599));
        pic.tick();
        assert_eq!(env.scheduler_lane_counts("poll")?, [3, 0]);
        pic.advance_time(Duration::from_secs(2));
        wait_for_poke_test(pic, "ordinary follow-on scan", || {
            Ok(env.state()?.observed_next_block > caught_up)
        })?;
        assert_eq!(env.scheduler_lane_counts("poll")?, [4, 0]);
        assert_eq!(env.subscriber_observation()?.pokes, 2);

        // A new idle request is independent of the preceding work and advances
        // the next deadline again.
        transfer(7)?;
        worker_poke(pic, backend, callers[1])?;
        env.wait_for_subscriber("later independent wake", |observation| {
            observation.pokes == 3
        })?;
        assert_eq!(env.scheduler_lane_counts("poll")?, [5, 0]);

        wait_for_poke_test(pic, "settled wake before a sequential request", || {
            Ok(env.scheduler_lane_state("poll")? == [false, true])
        })?;
        transfer(7)?;
        worker_poke(pic, backend, callers[0])?;
        env.wait_for_subscriber("sequential wake with no elapsed cooldown", |observation| {
            observation.pokes == 4
        })?;
        assert_eq!(env.scheduler_lane_counts("poll")?, [6, 0]);
    }
    Ok(())
}

#[test]
#[ignore = "uses real guarded polls and post-await trap cleanup"]
fn canonical_worker_pokes_respect_reserve_and_recover_after_trap() -> Result<()> {
    use std::time::Duration;
    let (env, callers) = canonical_worker_environment()?;
    let pic = &env.pic;
    let backend = env.event_horizon;
    let before = env.state()?.observed_next_block;
    env.set_scheduler_liquid_cycles_override(Some(900_000_000_000))?;
    env.set_balance(ICP_E8S)?;
    update::<_, ()>(pic, backend, "debug_start_schedulers", ())?;
    env.append_legacy_other("mint")?;
    for _ in 0..3 {
        worker_poke(pic, backend, callers[0])?;
    }
    wait_for_poke_test(pic, "funding under Reserve Protection", || {
        Ok(env.accepted_with_memo(1_347_768_404)? == 1
            && env.scheduler_lane_state("funding")? == [false, true])
    })?;
    assert_eq!(env.scheduler_lane_counts("poll")?, [0, 0]);
    assert_eq!(env.state()?.observed_next_block, before);
    assert_eq!(env.state()?.polling_mode, PollingMode::ReserveProtection);

    env.set_scheduler_liquid_cycles_override(Some(2_000_000_000_000))?;
    pic.advance_time(Duration::from_secs(3_601));
    wait_for_poke_test(pic, "ordinary reserve recovery", || {
        Ok(env.state()?.observed_next_block > before
            && env.scheduler_lane_state("poll")? == [false, true])
    })?;
    assert_eq!(env.state()?.polling_mode, PollingMode::Economy);
    assert_eq!(env.scheduler_lane_counts("poll")?, [1, 0]);

    env.append_account_admission_for(env.subscriber, 7, None, 10 * ICP_E8S)?;
    set_poke_historian_gate(&env, true)?;
    env.arm_scheduler_trap("poll")?;
    worker_poke(pic, backend, callers[0])?;
    wait_for_poke_test(pic, "poke-initiated scan waiting on Historian", || {
        let gate = poke_historian_gate(&env)?;
        Ok(gate.entered == 1 && gate.waiting)
    })?;
    worker_poke(pic, backend, callers[1])?;
    set_poke_historian_gate(&env, false)?;
    wait_for_poke_test(pic, "trap cleanup and delayed recovery timer", || {
        Ok(env.scheduler_lane_state("poll")? == [false, true])
    })?;
    assert!(env.public_logs()?.contains("forced poll scheduler trap"));
    assert_eq!(env.scheduler_lane_counts("poll")?, [2, 0]);
    pic.advance_time(Duration::from_secs(10));
    pic.tick();
    assert_eq!(env.scheduler_lane_counts("poll")?, [2, 0]);

    let restart_cursor = env.state()?.observed_next_block;
    env.append_legacy_other("mint")?;
    worker_poke(pic, backend, callers[1])?;
    wait_for_poke_test(pic, "legitimate wake after lane recovery", || {
        Ok(env.state()?.observed_next_block > restart_cursor
            && env.scheduler_lane_state("poll")? == [false, true])
    })?;
    assert_eq!(env.scheduler_lane_counts("poll")?, [3, 0]);
    Ok(())
}
