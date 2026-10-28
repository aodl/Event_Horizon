// Fixture-only seeding and inspection surround genuine audited production
// modules. No production principal, export, or financial control is substituted.
fn production_baseline_upgrade_fixed_anchors(generic: bool) -> Result<()> {
    let baseline_debug = baseline()?;
    let old = std::fs::read(
        workspace_root()?.join("tests/fixtures/event_horizon_81c5a0d_production.wasm"),
    )?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&old)),
        "2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528"
    );
    let candidate = if let Some(path) = std::env::var_os("EH_TEST_PRODUCTION_WASM") {
        std::fs::read(path)?
    } else {
        wasm(&EVENT_HORIZON_PROD_WASM, "event-horizon", None)?
    };
    for transfer_pending in [false, true] {
        let env = Env::with_anchors_at(
            baseline_debug.clone(),
            200_000_000_000_000,
            true,
            generic,
            false,
            true,
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
        env.admit(7, Some("1"), 10 * ICP_E8S)?;
        env.admit(7, Some("0.01"), 10 * ICP_E8S)?;
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
        let pending = env.state()?.cmc_state;
        assert!(pending.contains(if transfer_pending {
            "CmcTransferPending"
        } else {
            "NotifyPending"
        }));
        assert!(pending.contains("planned_surplus: Some(PlannedSurplus"));
        let self_before = encode_one(env.subscription(7)?)?;
        let external_before = encode_one(preview(&env, "A@B.7")?)?;
        let aliases_before = encode_one(query::<_, AliasMetadata>(
            &env.pic,
            env.event_horizon,
            "debug_alias_metadata",
            (),
        )?)?;
        let instance_before = encode_one(query::<_, InstanceInfo>(
            &env.pic,
            env.event_horizon,
            "get_instance",
            (),
        )?)?;
        let price_before = encode_one(env.state()?.pricing)?;
        let upgrade = |bytes: Vec<u8>| -> Result<()> {
            env.pic
                .upgrade_canister(env.event_horizon, bytes, candid::encode_args(())?, None)
                .map_err(|e| anyhow!("production upgrade: {e:?}"))?;
            Ok(())
        };
        // Run the reviewed canonical production baseline before the candidate.
        upgrade(old.clone())?;
        for _ in 0..60 {
            env.pic.tick();
        }
        assert_eq!(
            encode_one(query::<_, InstanceInfo>(
                &env.pic,
                env.event_horizon,
                "get_instance",
                ()
            )?)?,
            instance_before
        );
        upgrade(candidate.clone())?;
        for _ in 0..60 {
            env.pic.tick();
        }
        for _ in 0..2 {
            upgrade(baseline_debug.clone())?;
            let before = env.state()?;
            assert_eq!(before.cmc_state, pending);
            assert_eq!(encode_one(&before.pricing)?, price_before);
            assert_eq!(encode_one(env.subscription(7)?)?, self_before);
            assert_eq!(encode_one(preview(&env, "A@B.7")?)?, external_before);
            assert_eq!(
                encode_one(query::<_, AliasMetadata>(
                    &env.pic,
                    env.event_horizon,
                    "debug_alias_metadata",
                    ()
                )?)?,
                aliases_before
            );
            assert!(env.global_subscription()?);
            let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
            let first = before.observed_next_block;
            update::<_, u64>(
                &env.pic,
                env.observed_ledger,
                "debug_append_transfers",
                vec![
                    Append {
                        from: source.clone(),
                        to: account_id(env.cmc, numbered(7)),
                        amount_e8s: 1_999_999,
                        icrc1_memo: None,
                    },
                    Append {
                        from: source,
                        to: account_id(env.cmc, numbered(7)),
                        amount_e8s: 2_000_000,
                        icrc1_memo: None,
                    },
                ],
            )?;
            // A duplicate declaration must preserve the lower threshold. Its
            // admitting block remains separate from the observed generic stream.
            fund_suffix(&env, "A@B.7:0.03")?;
            let pokes = poke_count(&env, env.subscriber)?;
            upgrade(candidate.clone())?;
            env.wait_for_wakeup_pokes(env.subscriber, pokes + 1)?;
            let n = notification(&env, env.subscriber)?;
            assert!(n.global_activity);
            assert_eq!(
                n.matches,
                vec![PokeMatch {
                    subject: env.cmc,
                    target: WatchTarget::Subaccount(7),
                    first_block_index: first + 1,
                    max_amount: 2_000_000u64.into()
                }]
            );
            for _ in 0..40 {
                env.pic.tick();
            }
            upgrade(baseline_debug.clone())?;
            let after = env.state()?;
            assert_eq!(
                after.observed_next_block,
                first + if generic { 2 } else { 3 }
            );
            assert_eq!(
                after.admission_next_block,
                before.admission_next_block + if generic { 1 } else { 3 }
            );
            assert_eq!(after.cmc_state, pending);
        }
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        upgrade(candidate.clone())?;
        for _ in 0..100 {
            env.pic.tick();
        }
        upgrade(baseline_debug.clone())?;
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        assert_eq!(encode_one(env.subscription(7)?)?, self_before);
        assert_eq!(encode_one(preview(&env, "A@B.7")?)?, external_before);
    }
    Ok(())
}
#[test]
#[ignore = "reviewed canonical production baseline to candidate, ICP"]
fn reviewed_production_baseline_upgrade_icp() -> Result<()> {
    production_baseline_upgrade_fixed_anchors(false)
}
#[test]
#[ignore = "reviewed canonical production baseline to candidate, generic"]
fn reviewed_production_baseline_upgrade_generic() -> Result<()> {
    production_baseline_upgrade_fixed_anchors(true)
}
