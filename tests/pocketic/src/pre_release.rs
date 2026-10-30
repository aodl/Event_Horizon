// Reconstruct the exact observed live module; this is historical storage evidence,
// not a requirement to retain its obsolete vector notification ABI.
#[test]
#[ignore = "builds wasm and runs PocketIC"]
fn observed_54d8b4b_production_upgrades_preserving_financial_state() -> Result<()> {
    #[derive(CandidType, Deserialize, Debug)]
    struct HistoricalMetadata {
        admission_bootstrapped: bool,
        admission_next_block: u64,
        observed_bootstrapped: bool,
        observed_next_block: u64,
        polling_mode: PollingMode,
        last_health_log_day: Option<u64>,
    }
    let root = workspace_root()?;
    let seed = std::fs::read(root.join("tests/fixtures/event_horizon_54d8b4b_debug.wasm"))?;
    let old = std::fs::read(root.join("tests/fixtures/event_horizon_54d8b4b_production.wasm"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&seed)),
        "43fb4206372a42c1ef30238101fdc41d7ef44ccc22945461c6eed28e56bfef7c"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&old)),
        "e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca"
    );
    let candidate = std::fs::read(root.join("release-artifacts/event_horizon.wasm"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&candidate)),
        "f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970"
    );
    for surplus in [false, true] {
        for transfer_pending in [false, true] {
            let env = Env::with_anchors(
                seed.clone(),
                200_000_000_000_000,
                surplus,
                false,
                false,
                true,
            )?;
            env.set_scheduler_liquid_cycles_override(Some(0))?;
            env.price_once()?;
            env.poll()?;
            env.admit(7, Some("1"), 10 * ICP_E8S)?;
            env.admit(7, Some("0.01"), 10 * ICP_E8S)?;
            env.admit_global(1_000 * ICP_E8S)?;
            // Model a genuine earlier healthy low-water observation below
            // today's 200T actual balance. Otherwise ordinary startup funding
            // lowers the epoch minimum by its execution cost, even in old
            // production, making byte equality an invalid expectation.
            env.set_liquid_cycles_override(Some(150_000_000_000_000))?;
            if surplus {
                env.set_surplus_level(19)?;
            }
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
            // The historical CMC transfer appends a Ledger block. Consume it
            // with the historical reader before freezing cursor expectations;
            // otherwise production correctly catches up that outstanding block.
            env.poll()?;
            let before = env.state()?;
            assert!(before.cmc_state.contains(if transfer_pending {
                "CmcTransferPending"
            } else {
                "NotifyPending"
            }));
            if surplus {
                assert!(before
                    .surplus_policy
                    .contains("epoch_min_liquid_cycles: 150000000000000"));
            }
            assert_eq!(before.subscriptions, 1);
            assert_eq!(before.global_subscriptions, 1);
            assert_eq!(
                before
                    .cmc_state
                    .contains("planned_surplus: Some(PlannedSurplus"),
                surplus
            );
            assert_eq!(before.pricing.current.global_icp, 1_000);
            assert_eq!(before.pricing.current.range_icp, 20);
            let instance = encode_one(query::<_, InstanceInfo>(
                &env.pic,
                env.event_horizon,
                "get_instance",
                (),
            )?)?;
            let subscription = encode_one(env.subscription(7)?)?;
            let mut snapshot = effective_stable_snapshot(&env);
            snapshot.retain(|id, _| *id <= 8);
            // Debug init deliberately does not start schedulers and its liquid
            // override kept ReserveProtection. Genuine production post_upgrade
            // recomputes mode from the real 200T balance. Model that sole expected
            // metadata change explicitly; keep every cursor/health byte checked.
            let mut metadata: HistoricalMetadata = decode_one(&snapshot[&0][0].1)?;
            assert_eq!(metadata.polling_mode, PollingMode::ReserveProtection);
            metadata.polling_mode = PollingMode::Continuous;
            snapshot.get_mut(&0).unwrap()[0].1 = encode_one(metadata)?;
            for bytes in [old.clone(), candidate.clone()] {
                env.pic
                    .upgrade_canister(env.event_horizon, bytes, candid::encode_args(())?, None)
                    .map_err(|e| anyhow!("observed-version upgrade: {e:?}"))?;
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
                    instance
                );
                let mut after = effective_stable_snapshot(&env);
                after.retain(|id, _| *id <= 8);
                assert_effective_snapshots_equal(&snapshot, &after)?;
            }
            // Additive stores are inspected through a copy of stable memory.
            let additive = effective_stable_snapshot(&env);
            assert_eq!(
                additive[&9],
                vec![(
                    b"E".to_vec(),
                    Principal::from_text("eo6ei-gaaaa-aaaar-qchra-cai")?
                        .as_slice()
                        .to_vec()
                )]
            );
            let alias_metadata: AliasMetadata = decode_one(&additive[&10][0].1)?;
            assert_eq!(
                alias_metadata.revision,
                Some(vec![
                    0x89, 0xd9, 0xf2, 0x56, 0x01, 0x96, 0x2f, 0xb3, 0x94, 0xad, 0x9e, 0xbf, 0x34,
                    0xaa, 0xce, 0xcc, 0xf4, 0x6b, 0x83, 0x3b, 0x9c, 0xfd, 0x3e, 0xd2, 0xdc, 0x7e,
                    0xb9, 0x42, 0xe9, 0xd6, 0x36, 0x25
                ])
            );
            assert!(alias_metadata.last_success.is_some());
            assert_eq!(alias_metadata.last_success, alias_metadata.last_attempt);
            assert!(additive[&11].is_empty());
            // Keep the candidate production module running for notifications
            // and recovery; raw historical-region comparisons above already
            // inspected pending journals and cursors without a debug upgrade.
            let pokes_before = poke_count(&env, env.subscriber)?;
            let source = account_id(Principal::from_slice(&[91, 1]), [0; 32]);
            let at = env.append(
                source,
                account_id(env.subscriber, numbered(7)),
                2 * ICP_E8S,
                None,
            )?;
            env.wait_for_wakeup_pokes(env.subscriber, pokes_before + 1)?;
            let received = notification(&env, env.subscriber)?;
            assert!(received.global_activity);
            assert_eq!(
                received.matches,
                vec![PokeMatch {
                    subject: env.subscriber,
                    target: WatchTarget::Subaccount(7),
                    first_block_index: at,
                    max_amount: (2 * ICP_E8S).into(),
                }]
            );
            update::<_, ()>(
                &env.pic,
                env.ledger,
                "debug_set_legacy_behavior",
                LegacyBehavior::Normal,
            )?;
            update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
            // Let the actual production maintenance timer recover the same
            // journal. Advance simulated time, not the financial identity.
            env.pic.advance_time(std::time::Duration::from_secs(3_600));
            for _ in 0..100 {
                env.pic.tick();
            }
            env.upgrade_with_debug_wasm(wasm(
                &EVENT_HORIZON_WASM,
                "event-horizon",
                Some("debug_api"),
            )?)?;
            let recovered = env.state()?;
            assert_eq!(recovered.cmc_state, "Idle");
            assert!(recovered.admission_next_block > before.admission_next_block);
            assert!(recovered.observed_next_block > before.observed_next_block);
            assert_eq!(encode_one(env.subscription(7)?)?, subscription);
            assert!(env.global_subscription()?);
            assert_eq!(
                env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?,
                u64::from(surplus)
            );
        }
    }
    Ok(())
}
