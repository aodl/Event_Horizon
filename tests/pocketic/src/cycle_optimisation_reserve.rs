#[test]
#[ignore = "actual CDK sender cost at the reserve boundary, shadow wrapper only"]
#[cfg(feature = "optimisation_benchmarks")]
fn actual_sender_reserve_boundary() -> Result<()> {
    #[derive(CandidType)]
    struct Probe {
        subscriber: Principal,
        notification: PokeNotification,
        offset: i64,
        warm_global_cache: bool,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct ProbeResult {
        supplied_liquid: u128,
        actual_typed_cost: u128,
        actual_raw_cost: u128,
        payload_bytes: u64,
        accepted: bool,
        remaining_liquid: u128,
    }
    let old = baseline()?;
    let new =
        std::fs::read(std::env::var_os("EH_OPT_CANDIDATE").context("candidate shadow required")?)?;
    let mut csv=String::from("variant,hints,bits,warm,offset,supplied_liquid,typed_cost,raw_cost,payload_bytes,accepted,remaining_liquid\n");
    for (name, bytes) in [("baseline", old), ("candidate", new)] {
        for (hints, bits) in [(0u64, 0usize), (1, 257), (256, 4097), (256, 32769)] {
            if !selected_scenario("reserve", hints, bits as u64) {
                continue;
            }
            for warm in [false, true] {
                if hints != 0 && warm {
                    continue;
                }
                for offset in [-1i64, 0, 1] {
                    let env = Env::with_anchors_at(
                        bytes.clone(),
                        2_000_000_000_000_000,
                        false,
                        false,
                        false,
                        false,
                        Some(pocket_ic::Time::from_nanos_since_unix_epoch(
                            1_700_000_000_000_000_000,
                        )),
                    )?;
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
                    let subject = {
                        let mut p = [91u8; 29];
                        p[28] = 1;
                        Principal::from_slice(&p)
                    };
                    let notification = PokeNotification {
                        global_activity: true,
                        matches: (0..hints)
                            .map(|i| PokeMatch {
                                subject,
                                target: WatchTarget::Subaccount(u64::MAX - hints + 1 + i),
                                first_block_index: u64::MAX - i,
                                max_amount: candid::Nat(candid::Nat::from(1u8).0 << (bits - 1)),
                            })
                            .collect(),
                    };
                    let result: ProbeResult = update(
                        &env.pic,
                        env.event_horizon,
                        "debug_probe_sender_reserve",
                        Probe {
                            subscriber: env.subscriber,
                            notification: notification.clone(),
                            offset,
                            warm_global_cache: warm,
                        },
                    )?;
                    assert_eq!(result.actual_typed_cost, result.actual_raw_cost);
                    assert_eq!(
                        result.payload_bytes,
                        encode_one(&notification)?.len() as u64
                    );
                    assert_eq!(
                        result.supplied_liquid,
                        1_000_000_000_000
                            + result.actual_typed_cost
                            + if offset < 0 { 0 } else { offset as u128 }
                            - if offset < 0 { 1 } else { 0 }
                    );
                    assert_eq!(
                        result.accepted,
                        offset >= 0,
                        "actual cost/reserve rule changed: {result:?}"
                    );
                    for _ in 0..40 {
                        env.pic.tick();
                    }
                    assert_eq!(
                        poke_count(&env, env.subscriber)?,
                        u64::from(result.accepted)
                    );
                    if result.accepted {
                        assert_eq!(
                            query::<_, Option<Principal>>(
                                &env.pic,
                                env.subscriber,
                                "debug_last_caller",
                                ()
                            )?,
                            Some(env.event_horizon)
                        );
                        // Use the independent receiver digest for maximum records,
                        // keeping its ordinary decoder and query budget unchanged.
                        let digest: Vec<u8> =
                            update(&env.pic, env.subscriber, "debug_notification_digest", ())?;
                        assert_eq!(digest, Sha256::digest(encode_one(&notification)?).to_vec());
                    }
                    csv.push_str(&format!(
                        "{name},{hints},{bits},{warm},{offset},{},{},{},{},{},{}\n",
                        result.supplied_liquid,
                        result.actual_typed_cost,
                        result.actual_raw_cost,
                        result.payload_bytes,
                        result.accepted,
                        result.remaining_liquid
                    ));
                }
            }
        }
    }
    save_complete("EH_OPT_OUTPUT", &csv)?;
    println!("{csv}");
    Ok(())
}
