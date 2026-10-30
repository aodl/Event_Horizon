// These fixtures deliberately modify mutable inputs while one admission awaits
// Historian. The reference is separately installed, verified baseline Wasm.
#[test]
#[ignore = "baseline/candidate mutable alias and price reads across a controlled await"]
fn mutable_alias_and_price_reads_keep_their_logical_admission_points() -> Result<()> {
    #[derive(Clone, Debug, CandidType, Deserialize)]
    struct ObservedRoute {
        destination: Principal,
        memo: Vec<u8>,
    }
    #[derive(CandidType, Deserialize)]
    struct RouteGate {
        entered: u64,
        waiting: bool,
        routes: Vec<ObservedRoute>,
    }
    // The real Historian method is a replicated query and cannot await. This
    // explicit test-only update wrapper pauses before the same pure response.
    // It establishes read/commit ordering, not production Hist instruction cost.
    wasm(&HISTORIAN_WASM, "mock-historian", Some("interleaving_gate"))?;
    let candidate = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
    let mut reference = None;
    for bytes in [baseline()?, candidate] {
        let env = Env::with_anchors_at(
            bytes,
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
        let aliases = vec![CanisterAlias {
            alias: "A".into(),
            canister_id: env.subscriber,
        }];
        set_aliases(&env, aliases.clone(), "", None)?;
        refresh_aliases(&env)?;
        let pricing = env.pricing()?;
        // Establish and freeze the lower, announced next-epoch price.
        env.pic
            .set_time(pocket_ic::Time::from_nanos_since_unix_epoch(
                (pricing.next_freeze_at - 86_400) * 1_000_000_000,
            ));
        update::<_, ()>(&env.pic, env.cmc, "debug_set_rate", 20_000u64)?;
        env.price_once()?;
        env.pic.advance_time(std::time::Duration::from_secs(86_400));
        env.price_once()?;
        let frozen = env.pricing()?;
        assert_eq!(
            frozen.next.context("missing announced price")?.account_icp,
            5
        );
        env.pic
            .set_time(pocket_ic::Time::from_nanos_since_unix_epoch(
                (frozen.next_effective_at - 1) * 1_000_000_000,
            ));
        for memo in [b"A.7".as_slice(), b"B.8", b"Y.9"] {
            env.set_route(memo.to_vec(), 500_000_000)?;
            env.append(
                account_id(env.faucet, [0; 32]),
                account_id(env.event_horizon, [0; 32]),
                1,
                Some(memo.to_vec()),
            )?;
        }
        let before = env.state()?.observed_next_block;
        update::<_, ()>(&env.pic, env.historian, "debug_route_gate", true)?;
        let poll = env
            .pic
            .submit_call(
                env.event_horizon,
                Principal::anonymous(),
                "debug_poll_once",
                encode_one(())?,
            )
            .map_err(|e| anyhow!("submit {e:?}"))?;
        let mut paused = false;
        for _ in 0..20 {
            env.pic.tick();
            let g: RouteGate = query(&env.pic, env.historian, "debug_route_gate_state", ())?;
            if g.entered == 1 && g.waiting {
                paused = true;
                break;
            }
        }
        if !paused {
            let g: RouteGate = query(&env.pic, env.historian, "debug_route_gate_state", ())?;
            eprintln!(
                "gate entered={} waiting={} routes={:?} state={:?}",
                g.entered,
                g.waiting,
                g.routes,
                env.state()?
            );
            for p in [env.historian, env.event_horizon] {
                for log in env
                    .pic
                    .fetch_canister_logs(p, Principal::anonymous())
                    .map_err(|e| anyhow!("{e:?}"))?
                {
                    eprintln!(
                        "gate canister={p} log={}",
                        String::from_utf8_lossy(&log.content)
                    );
                }
            }
        }
        assert!(paused, "did not reach the first exact-route await");
        assert_eq!(
            env.state()?.observed_next_block,
            before,
            "page published across await"
        );
        // First admission has already captured old price 10. Later blocks must
        // independently resolve newly published aliases and read new price 5.
        env.pic.advance_time(std::time::Duration::from_secs(2));
        env.price_once()?;
        assert_eq!(env.pricing()?.current.account_icp, 5);
        let mut published = aliases;
        published.push(CanisterAlias {
            alias: "B".into(),
            canister_id: env.subscriber_second,
        });
        set_aliases(&env, published.clone(), "", None)?;
        refresh_aliases(&env)?;
        update::<_, ()>(&env.pic, env.historian, "debug_route_gate", false)?;
        env.pic
            .await_call(poll)
            .map_err(|e| anyhow!("await {e:?}"))?;
        assert!(
            env.subscription(7)?.is_none(),
            "old-price decision was retroactively relaxed"
        );
        let second: Option<Subscription> = query(
            &env.pic,
            env.event_horizon,
            "debug_subscription",
            DebugSubscriptionArgs {
                subscriber: env.subscriber_second,
                target: WatchTarget::Subaccount(8),
            },
        )?;
        assert!(
            second.is_some(),
            "later block did not see new alias and price"
        );
        assert_eq!(env.state()?.subscriptions, 1);
        assert_eq!(env.state()?.observed_next_block, before + 3);
        let g: RouteGate = query(&env.pic, env.historian, "debug_route_gate_state", ())?;
        assert_eq!(g.entered, 2, "unknown alias must not call Historian");
        assert_eq!(
            g.routes
                .iter()
                .map(|r| r.memo.as_slice())
                .collect::<Vec<_>>(),
            vec![b"A.7".as_slice(), b"B.8"]
        );
        assert!(g.routes.iter().all(|r| r.destination == env.event_horizon));
        published.push(CanisterAlias {
            alias: "Y".into(),
            canister_id: env.subscriber,
        });
        set_aliases(&env, published, "", None)?;
        refresh_aliases(&env)?;
        env.poll()?;
        assert!(
            env.subscription(9)?.is_none(),
            "consumed unknown alias replayed later"
        );
        let semantic = encode_one((env.state()?, second, g.routes))?;
        if let Some(ref old) = reference {
            assert_eq!(old, &semantic, "mutable-input differential");
        } else {
            reference = Some(semantic);
        }
    }
    Ok(())
}
