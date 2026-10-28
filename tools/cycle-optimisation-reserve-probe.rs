// Generated shadow only: directly exercises the unchanged production sender
// with actual IC liquidity and actual typed/raw CDK cost. Never canonical.
#[derive(CandidType, Deserialize)]
struct PokeProbe {
    subscriber: candid::Principal,
    notification: crate::clients::subscriber::PokeNotification,
    offset: i64,
    warm_global_cache: bool,
}
#[derive(CandidType, Deserialize)]
struct PokeProbeResult {
    supplied_liquid: u128,
    actual_typed_cost: u128,
    actual_raw_cost: u128,
    payload_bytes: u64,
    accepted: bool,
    remaining_liquid: u128,
}
#[ic_cdk::update]
fn debug_probe_sender_reserve(args: PokeProbe) -> PokeProbeResult {
    use ic_cdk::call::Call;
    assert!(matches!(args.offset, -1..=1));
    let encoded = candid::encode_one(&args.notification).expect("encode probe record");
    let cost = Call::bounded_wait(args.subscriber, "poke")
        .with_arg(&args.notification)
        .get_cost();
    let raw_cost = Call::bounded_wait(args.subscriber, "poke")
        .with_raw_args(&encoded)
        .get_cost();
    assert_eq!(cost, raw_cost);
    let required = crate::config::RESERVE_PROTECTION_CYCLES + cost;
    let target = if args.offset < 0 {
        required - 1
    } else {
        required + args.offset as u128
    };
    let current = ic_cdk::api::canister_liquid_cycle_balance();
    assert!(current > target, "probe needs fresh ample funding");
    assert_eq!(ic_cdk::api::cycles_burn(current - target), current - target);
    let supplied_liquid = ic_cdk::api::canister_liquid_cycle_balance();
    assert_eq!(supplied_liquid, target);
    // SEND_PROBE
    PokeProbeResult {
        supplied_liquid,
        actual_typed_cost: cost,
        actual_raw_cost: raw_cost,
        payload_bytes: encoded.len() as u64,
        accepted,
        remaining_liquid: ic_cdk::api::canister_liquid_cycle_balance(),
    }
}
