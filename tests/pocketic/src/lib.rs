#![allow(dead_code)]
#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        process::Command,
        sync::OnceLock,
    };

    use anyhow::{anyhow, bail, Context, Result};
    use candid::{decode_one, encode_one, CandidType, Deserialize, Principal};
    use pocket_ic::{CreateCanisterParams, PocketIc, PocketIcBuilder};
    use sha2::{Digest, Sha224};

    static LEDGER_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static HISTORIAN_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static CMC_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static SUBSCRIBER_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static EVENT_HORIZON_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static EVENT_HORIZON_PROD_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static FRONTEND_WASM: OnceLock<Vec<u8>> = OnceLock::new();

    fn workspace_root() -> Result<PathBuf> {
        for dir in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
            if dir.join("Cargo.toml").exists() && dir.join("SPEC.md").exists() {
                return Ok(dir.to_path_buf());
            }
        }
        bail!("workspace root not found")
    }

    fn wasm(cache: &OnceLock<Vec<u8>>, package: &str, features: Option<&str>) -> Result<Vec<u8>> {
        if let Some(bytes) = cache.get() {
            return Ok(bytes.clone());
        }
        let root = workspace_root()?;
        let mut cmd = Command::new("cargo");
        cmd.args([
            "build",
            "--locked",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "-p",
            package,
        ]);
        if let Some(features) = features {
            cmd.args(["--features", features]);
        }
        let status = cmd
            .current_dir(&root)
            .status()
            .with_context(|| format!("build {package}"))?;
        if !status.success() {
            bail!("wasm build failed for {package}")
        }
        let path = root.join(format!(
            "target/wasm32-unknown-unknown/release/{}.wasm",
            package.replace('-', "_")
        ));
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let _ = cache.set(bytes.clone());
        Ok(bytes)
    }

    fn update<A: CandidType, R: for<'de> Deserialize<'de> + CandidType>(
        pic: &PocketIc,
        id: Principal,
        method: &str,
        arg: A,
    ) -> Result<R> {
        let bytes = pic
            .update_call(id, Principal::anonymous(), method, encode_one(arg)?)
            .map_err(|e| anyhow!("{method}: {e:?}"))?;
        Ok(decode_one(&bytes)?)
    }
    fn query<A: CandidType, R: for<'de> Deserialize<'de> + CandidType>(
        pic: &PocketIc,
        id: Principal,
        method: &str,
        arg: A,
    ) -> Result<R> {
        let bytes = pic
            .query_call(id, Principal::anonymous(), method, encode_one(arg)?)
            .map_err(|e| anyhow!("{method}: {e:?}"))?;
        Ok(decode_one(&bytes)?)
    }

    fn account_id(owner: Principal, subaccount: [u8; 32]) -> Vec<u8> {
        let mut h = Sha224::new();
        h.update(b"\x0Aaccount-id");
        h.update(owner.as_slice());
        h.update(subaccount);
        let hash = h.finalize();
        let mut out = [0u8; 32];
        out[..4].copy_from_slice(&crc32fast::hash(&hash).to_be_bytes());
        out[4..].copy_from_slice(&hash);
        out.to_vec()
    }
    fn numbered(n: u8) -> [u8; 32] {
        let mut s = [0u8; 32];
        s[31] = n;
        s
    }

    #[derive(CandidType, Deserialize)]
    struct DebugInitArgs {
        ledger_canister: Principal,
        cmc_canister: Principal,
        historian_canister: Principal,
        faucet_canister: Principal,
    }
    #[derive(CandidType, Deserialize)]
    struct SetRoute {
        destination_canister_id: Principal,
        memo: Vec<u8>,
        total_e8s: u64,
    }
    #[derive(CandidType, Deserialize)]
    struct Append {
        from: Vec<u8>,
        to: Vec<u8>,
        amount_e8s: u64,
        icrc1_memo: Option<Vec<u8>>,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct DebugState {
        bootstrapped: bool,
        next_block: u64,
        polling_mode: PollingMode,
        subscriptions: u64,
        global_subscriptions: u64,
        cmc_state: String,
        pricing: Pricing,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct ValidatedCoreDebugState {
        bootstrapped: bool,
        next_block: u64,
        polling_mode: PollingMode,
        subscriptions: u64,
        cmc_state: String,
    }
    #[derive(CandidType, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
    struct Price {
        account_icp: u64,
        global_icp: u64,
    }
    #[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
    struct Pricing {
        initialized: bool,
        current: Price,
        current_effective_at: u64,
        next: Option<Price>,
        next_effective_at: u64,
        next_freeze_at: u64,
        observed_floor_xdr_permyriad: u64,
        floor_observed_at: u64,
        latest_xdr_permyriad: u64,
        latest_observed_at: u64,
        next_carried_forward_due_to_stale_rate: bool,
    }
    #[derive(CandidType, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
    enum PollingMode {
        ReserveProtection,
        Economy,
        Standard,
        Fast,
        VeryFast,
        Continuous,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct Subscription {
        subscriber: Principal,
        numbered_subaccount: u8,
        minimum_e8s: u64,
    }
    #[derive(CandidType, Deserialize)]
    struct DebugSubscriptionArgs {
        subscriber: Principal,
        subaccount: u8,
    }
    #[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
    struct Account {
        owner: Principal,
        subaccount: Option<Vec<u8>>,
    }
    #[derive(CandidType, Deserialize)]
    struct SetBalance {
        account: Account,
        e8s: u64,
    }
    #[derive(CandidType, Deserialize)]
    struct HttpRequest {
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        certificate_version: Option<u16>,
    }
    #[derive(CandidType, Deserialize)]
    struct HttpResponse {
        status_code: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        upgrade: Option<bool>,
    }
    #[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
    enum CmcBehavior {
        Ok,
        Refunded,
        Processing,
        TransactionTooOld,
        InvalidTransaction,
        Other,
    }
    #[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
    enum LegacyBehavior {
        Normal,
        DropResponseAfterAccept,
        BadFee,
        InsufficientFunds,
        TooOld,
        CreatedInFuture,
    }

    struct Env {
        pic: PocketIc,
        ledger: Principal,
        historian: Principal,
        cmc: Principal,
        subscriber: Principal,
        event_horizon: Principal,
        faucet: Principal,
    }
    impl Env {
        fn new() -> Result<Self> {
            let env = Self::new_unpriced()?;
            env.price_once()?;
            Ok(env)
        }
        fn new_unpriced() -> Result<Self> {
            Self::new_unpriced_with_cycles(200_000_000_000_000)
        }
        fn new_unpriced_with_cycles(event_horizon_cycles: u128) -> Result<Self> {
            Self::with_event_horizon_wasm(
                wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?,
                event_horizon_cycles,
            )
        }
        fn validated_baseline() -> Result<Self> {
            Self::with_event_horizon_wasm(
                include_bytes!("../../fixtures/event_horizon_core_validated_c48778d_debug.wasm")
                    .to_vec(),
                200_000_000_000_000,
            )
        }
        fn with_event_horizon_wasm(
            event_horizon_wasm: Vec<u8>,
            event_horizon_cycles: u128,
        ) -> Result<Self> {
            let pic = PocketIcBuilder::new().with_application_subnet().build();
            let ledger = pic.create_canister();
            let historian = pic.create_canister();
            let cmc = pic.create_canister();
            let subscriber = pic.create_canister();
            let event_horizon = pic
                .create_canister_with_params(
                    None,
                    CreateCanisterParams {
                        cycles: Some(event_horizon_cycles),
                        settings: None,
                        placement: None,
                    },
                )
                .map_err(|error| anyhow!("create Event Horizon canister: {error}"))?;
            for id in [ledger, historian, cmc, subscriber] {
                pic.add_cycles(id, 200_000_000_000_000);
            }
            pic.install_canister(
                ledger,
                wasm(&LEDGER_WASM, "mock-icp-ledger", None)?,
                vec![],
                None,
            );
            pic.install_canister(
                historian,
                wasm(&HISTORIAN_WASM, "mock-historian", None)?,
                vec![],
                None,
            );
            pic.install_canister(cmc, wasm(&CMC_WASM, "mock-cmc", None)?, vec![], None);
            pic.install_canister(
                subscriber,
                wasm(&SUBSCRIBER_WASM, "mock-subscriber", None)?,
                vec![],
                None,
            );
            let faucet = Principal::from_slice(&[42]);
            pic.install_canister(
                event_horizon,
                event_horizon_wasm,
                encode_one(DebugInitArgs {
                    ledger_canister: ledger,
                    cmc_canister: cmc,
                    historian_canister: historian,
                    faucet_canister: faucet,
                })?,
                None,
            );
            Ok(Self {
                pic,
                ledger,
                historian,
                cmc,
                subscriber,
                event_horizon,
                faucet,
            })
        }
        fn poll(&self) -> Result<()> {
            update::<_, ()>(&self.pic, self.event_horizon, "debug_poll_once", ())
        }
        fn fund(&self) -> Result<()> {
            update::<_, ()>(&self.pic, self.event_horizon, "debug_funding_once", ())
        }
        fn price_once(&self) -> Result<()> {
            update::<_, ()>(&self.pic, self.event_horizon, "debug_pricing_once", ())
        }
        fn pricing(&self) -> Result<Pricing> {
            query(&self.pic, self.event_horizon, "get_pricing", ())
        }
        fn append(
            &self,
            from: Vec<u8>,
            to: Vec<u8>,
            amount: u64,
            memo: Option<Vec<u8>>,
        ) -> Result<u64> {
            update(
                &self.pic,
                self.ledger,
                "debug_append_transfer",
                Append {
                    from,
                    to,
                    amount_e8s: amount,
                    icrc1_memo: memo,
                },
            )
        }
        fn state(&self) -> Result<DebugState> {
            query(&self.pic, self.event_horizon, "debug_state", ())
        }
        fn set_route(&self, memo: Vec<u8>, total: u64) -> Result<()> {
            update(
                &self.pic,
                self.historian,
                "debug_set_route",
                SetRoute {
                    destination_canister_id: self.event_horizon,
                    memo,
                    total_e8s: total,
                },
            )
        }
        fn subscription(&self, subaccount: u8) -> Result<Option<Subscription>> {
            query(
                &self.pic,
                self.event_horizon,
                "debug_subscription",
                DebugSubscriptionArgs {
                    subscriber: self.subscriber,
                    subaccount,
                },
            )
        }
        fn global_subscription(&self) -> Result<bool> {
            query(
                &self.pic,
                self.event_horizon,
                "debug_global_subscription",
                self.subscriber,
            )
        }
        fn admit_global(&self, total: u64) -> Result<Vec<u8>> {
            let memo = self.subscriber.to_text().replace('-', "").into_bytes();
            self.set_route(memo.clone(), total)?;
            self.append(
                account_id(self.faucet, [0; 32]),
                account_id(self.event_horizon, [0; 32]),
                1,
                Some(memo.clone()),
            )?;
            self.poll()?;
            Ok(memo)
        }
        fn admit(&self, subaccount: u8, threshold: Option<&str>, total: u64) -> Result<Vec<u8>> {
            let compact = self.subscriber.to_text().replace('-', "");
            let memo = match threshold {
                Some(v) => format!("{compact}.{subaccount}:{v}"),
                None => format!("{compact}.{subaccount}"),
            }
            .into_bytes();
            self.set_route(memo.clone(), total)?;
            self.append(
                account_id(self.faucet, [0; 32]),
                account_id(self.event_horizon, [0; 32]),
                10_000_000,
                Some(memo.clone()),
            )?;
            self.poll()?;
            Ok(memo)
        }
        fn upgrade_same_debug_wasm(&self) -> Result<()> {
            let controller = self
                .pic
                .get_controllers(self.event_horizon)
                .first()
                .copied()
                .ok_or_else(|| anyhow!("no controller"))?;
            self.pic
                .upgrade_canister(
                    self.event_horizon,
                    wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?,
                    encode_one(())?,
                    Some(controller),
                )
                .map_err(|e| anyhow!("upgrade event horizon: {e:?}"))?;
            Ok(())
        }
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn admission_and_multi_subaccount_matches_coalesce_to_one_poke() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        assert!(env.state()?.bootstrapped);
        env.admit(7, None, 1_000_000_000)?;
        env.admit(8, Some("0.01"), 1_000_000_000)?;
        assert_eq!(
            env.subscription(7)?
                .expect("unfiltered admitted")
                .minimum_e8s,
            0
        );
        assert_eq!(
            env.subscription(8)?
                .expect("thresholded admitted")
                .minimum_e8s,
            1_000_000
        );

        let source = account_id(Principal::from_slice(&[9]), [0; 32]);
        env.append(
            source.clone(),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.append(
            source.clone(),
            account_id(env.subscriber, numbered(8)),
            999_999,
            None,
        )?;
        env.append(
            source,
            account_id(env.subscriber, numbered(8)),
            1_000_000,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            1
        );
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7, 8]
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn admission_requires_faucet_and_complete_ten_icp_historian_evidence() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let compact = env.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7").into_bytes();
        env.set_route(memo.clone(), 999_999_999)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo.clone()),
        )?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_none(),
            "below 10 ICP does not admit"
        );

        env.set_route(memo.clone(), 1_000_000_000)?;
        update::<_, ()>(&env.pic, env.historian, "debug_set_complete", false)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo.clone()),
        )?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_none(),
            "incomplete Historian does not admit"
        );

        update::<_, ()>(&env.pic, env.historian, "debug_set_complete", true)?;
        update::<_, ()>(&env.pic, env.historian, "debug_set_fault", true)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo.clone()),
        )?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_none(),
            "faulted Historian does not admit"
        );

        update::<_, ()>(&env.pic, env.historian, "debug_set_fault", false)?;
        let stranger = Principal::from_slice(&[99]);
        env.append(
            account_id(stranger, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo.clone()),
        )?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_none(),
            "non-Faucet transfer never admits"
        );

        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_some(),
            "later perpetual payout admits after evidence recovers"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn global_admission_and_poke_precedence_are_enforced() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;

        env.admit_global(9_999_999_999)?;
        assert!(
            !env.global_subscription()?,
            "global price is enforced separately"
        );

        let global_memo = env.subscriber.to_text().replace('-', "").into_bytes();
        env.set_route(global_memo.clone(), 10_000_000_000)?;
        env.append(
            account_id(Principal::from_slice(&[99]), [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(global_memo),
        )?;
        env.poll()?;
        assert!(
            !env.global_subscription()?,
            "a forged non-Faucet transfer cannot admit"
        );

        env.admit_global(10_000_000_000)?;
        assert!(env.global_subscription()?);
        env.admit(7, None, 1_000_000_000)?;

        let before = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;
        env.poll()?;
        for _ in 0..3 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            before,
            "an empty poll produces no global poke"
        );

        let source = account_id(Principal::from_slice(&[9]), [0; 32]);
        let unrelated = account_id(Principal::from_slice(&[8]), [0; 32]);
        env.append(source.clone(), unrelated.clone(), 1, None)?;
        env.append(source.clone(), unrelated, 1, None)?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            before + 1,
            "many transactions coalesce into one global poke"
        );
        assert!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?.is_empty()
        );

        env.append(
            source.clone(),
            account_id(Principal::from_slice(&[6]), [0; 32]),
            1,
            None,
        )?;
        env.append(source, account_id(env.subscriber, numbered(7)), 1, None)?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            before + 2
        );
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7],
            "specific account information takes precedence over the global empty hint"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn pricing_initialization_failure_recovery_freeze_and_activation() -> Result<()> {
        let env = Env::new_unpriced()?;
        update::<_, ()>(&env.pic, env.cmc, "debug_set_pricing_fail", true)?;
        env.price_once()?;
        assert!(!env.pricing()?.initialized);

        env.pic.advance_time(std::time::Duration::from_secs(86_400));
        update::<_, ()>(&env.pic, env.cmc, "debug_set_pricing_fail", false)?;
        update::<_, ()>(&env.pic, env.cmc, "debug_set_rate", 10_000u64)?;
        env.price_once()?;
        let initialized = env.pricing()?;
        assert_eq!(
            initialized.current,
            Price {
                account_icp: 10,
                global_icp: 100
            }
        );
        update::<_, ()>(&env.pic, env.cmc, "debug_set_rate", 30_000u64)?;
        env.price_once()?;
        assert_eq!(
            env.pricing()?.latest_xdr_permyriad,
            10_000,
            "only one successful observation is stored per UTC date"
        );

        // Move to a daily observation before the currently prepared freeze. If launch
        // occurred in the final week, activate that carried epoch first.
        let mut pricing = initialized;
        let mut now = env.pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
        if pricing.next_freeze_at <= now + 86_400 {
            env.pic.advance_time(std::time::Duration::from_secs(
                pricing.next_effective_at - now,
            ));
            env.price_once()?;
            pricing = env.pricing()?;
            now = env.pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
        }
        let observation_at = pricing.next_freeze_at - 86_400;
        env.pic
            .advance_time(std::time::Duration::from_secs(observation_at - now));
        update::<_, ()>(&env.pic, env.cmc, "debug_set_rate", 20_000u64)?;
        env.price_once()?;

        env.pic.advance_time(std::time::Duration::from_secs(86_400));
        env.price_once()?;
        let frozen = env.pricing()?;
        assert_eq!(
            frozen.next,
            Some(Price {
                account_icp: 5,
                global_icp: 50
            })
        );
        assert!(!frozen.next_carried_forward_due_to_stale_rate);

        update::<_, ()>(&env.pic, env.cmc, "debug_set_rate", 50_000u64)?;
        env.pic.advance_time(std::time::Duration::from_secs(86_400));
        env.price_once()?;
        assert_eq!(
            env.pricing()?.next,
            frozen.next,
            "the announced price is immutable"
        );

        let now = env.pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
        env.pic.advance_time(std::time::Duration::from_secs(
            frozen.next_effective_at - now,
        ));
        env.price_once()?;
        assert_eq!(env.pricing()?.current, frozen.next.unwrap());

        env.poll()?;
        env.admit(7, None, 499_999_999)?;
        assert!(
            env.subscription(7)?.is_none(),
            "N-1 e8s does not meet the dynamic price"
        );
        env.admit(7, None, 500_000_000)?;
        assert!(
            env.subscription(7)?.is_some(),
            "N e8s meets the dynamic price"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn stale_rate_at_freeze_carries_current_prices_forward() -> Result<()> {
        let env = Env::new_unpriced()?;
        let original_timestamp = env.pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_rate_timestamp",
            original_timestamp,
        )?;
        env.price_once()?;
        let first = env.pricing()?;

        // Reach the following epoch so its freeze is more than seven days after
        // the deliberately fixed CMC observation timestamp.
        let mut now = original_timestamp;
        env.pic.advance_time(std::time::Duration::from_secs(
            first.next_effective_at - now,
        ));
        env.price_once()?;
        let preparing = env.pricing()?;
        now = env.pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
        env.pic.advance_time(std::time::Duration::from_secs(
            preparing.next_freeze_at - now,
        ));
        env.price_once()?;
        let frozen = env.pricing()?;
        assert_eq!(frozen.next, Some(frozen.current));
        assert!(frozen.next_carried_forward_due_to_stale_rate);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn pricing_observation_skips_the_day_before_spending_into_reserve() -> Result<()> {
        let env = Env::new_unpriced_with_cycles(1_030_000_000_000)?;
        env.price_once()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.cmc, "debug_pricing_calls", ())?,
            0,
            "the CMC rate call must not be issued when its cost would encroach on reserve"
        );
        assert!(!env.pricing()?.initialized);

        env.pic.add_cycles(env.event_horizon, 200_000_000_000_000);
        env.price_once()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.cmc, "debug_pricing_calls", ())?,
            0,
            "a reserve skip consumes the day's single observation opportunity"
        );

        env.pic.advance_time(std::time::Duration::from_secs(86_400));
        env.price_once()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.cmc, "debug_pricing_calls", ())?,
            1
        );
        assert!(env.pricing()?.initialized);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn subscriber_trap_does_not_hold_reader_cursor() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.admit(7, None, 1_000_000_000)?;
        update::<_, ()>(&env.pic, env.subscriber, "debug_set_trap", true)?;
        let source = account_id(Principal::from_slice(&[9]), [0; 32]);
        env.append(
            source.clone(),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        let after_failed = env.state()?.next_block;
        update::<_, ()>(&env.pic, env.subscriber, "debug_set_trap", false)?;
        env.append(source, account_id(env.subscriber, numbered(7)), 1, None)?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert!(
            env.state()?.next_block > after_failed,
            "reader continues after disposable poke failure"
        );
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            1,
            "only successful poke commits in trap mock"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn archived_prefix_is_skipped_and_live_reader_continues() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let source = account_id(Principal::from_slice(&[8]), [0; 32]);
        let destination = account_id(Principal::from_slice(&[9]), [0; 32]);
        for _ in 0..5 {
            env.append(source.clone(), destination.clone(), 1_000_000, None)?;
        }
        update::<_, ()>(&env.pic, env.ledger, "debug_set_first_local_block", 3u64)?;
        env.poll()?;
        assert_eq!(env.state()?.next_block, 5);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn unexplained_ledger_hole_preserves_cursor_until_archive_evidence_arrives() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let source = account_id(Principal::from_slice(&[8]), [0; 32]);
        let destination = account_id(Principal::from_slice(&[9]), [0; 32]);
        for _ in 0..5 {
            env.append(source.clone(), destination.clone(), 1_000_000, None)?;
        }
        update::<_, ()>(&env.pic, env.ledger, "debug_set_first_local_block", 3u64)?;
        update::<_, ()>(&env.pic, env.ledger, "debug_suppress_archive_info", true)?;
        env.poll()?;
        assert_eq!(
            env.state()?.next_block,
            0,
            "unexplained hole must be retried"
        );

        update::<_, ()>(&env.pic, env.ledger, "debug_suppress_archive_info", false)?;
        env.poll()?;
        assert_eq!(env.state()?.next_block, 5);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn transaction_after_pinned_boundary_waits_for_following_poll() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let memo = env.admit(7, None, 1_000_000_000)?;

        // Put a qualifying Faucet transfer at the head so processing the pinned page
        // reaches the existing Historian await after the Ledger response is captured.
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        let poll = env
            .pic
            .submit_call(
                env.event_horizon,
                Principal::anonymous(),
                "debug_poll_once",
                encode_one(())?,
            )
            .map_err(|e| anyhow!("submit poll: {e:?}"))?;
        // First round starts the poll; second executes the Ledger query and fixes its
        // chain_length response before this transaction is appended.
        env.pic.tick();
        env.pic.tick();

        env.append(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.pic
            .await_call(poll)
            .map_err(|e| anyhow!("await poll: {e:?}"))?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            0,
            "transaction beyond the pinned boundary must not be poked in this poll"
        );

        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            1,
            "following poll must process the deferred transaction"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn funding_sweep_uses_legacy_transfer_and_reaches_cmc() -> Result<()> {
        let env = Env::new()?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: env.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        env.fund()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.ledger, "debug_accepted_legacy_transfers", ())?,
            1
        );
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 1);
        assert!(env.state()?.cmc_state.contains("Idle"));
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn lost_legacy_transfer_response_reuses_identity_without_second_spend() -> Result<()> {
        let env = Env::new()?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: env.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::DropResponseAfterAccept,
        )?;
        env.fund()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.ledger, "debug_accepted_legacy_transfers", ())?,
            1
        );
        assert!(env.state()?.cmc_state.contains("TransferPending"));
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.ledger, "debug_accepted_legacy_transfers", ())?,
            1,
            "duplicate recovery must not spend twice"
        );
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 1);
        assert!(env.state()?.cmc_state.contains("Idle"));
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn expired_transfer_identity_clears_and_later_replans() -> Result<()> {
        let env = Env::new()?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: env.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::CreatedInFuture,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("TransferPending"));

        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::TooOld,
        )?;
        env.fund()?;
        assert!(
            env.state()?.cmc_state.contains("Idle"),
            "expired identity must restore autonomous funding liveness"
        );

        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.ledger, "debug_accepted_legacy_transfers", ())?,
            1
        );
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 1);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn successful_cmc_top_up_immediately_accelerates_polling_mode() -> Result<()> {
        let env = Env::new()?;
        assert_eq!(env.state()?.polling_mode, PollingMode::ReserveProtection);
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: env.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("NotifyPending"));
        assert_eq!(env.state()?.polling_mode, PollingMode::ReserveProtection);

        // PocketIC cannot let the mock CMC mint cycles. Model the real CMC side effect
        // before returning Success, then prove the funding callback reschedules immediately.
        env.pic.add_cycles(env.event_horizon, 20_000_000_000_000);
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert_eq!(env.state()?.polling_mode, PollingMode::Continuous);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn cmc_processing_survives_upgrade_and_resumes_same_notification() -> Result<()> {
        let env = Env::new()?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: env.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("NotifyPending"));
        env.upgrade_same_debug_wasm()?;
        assert!(
            env.state()?.cmc_state.contains("NotifyPending"),
            "financial state survives upgrade"
        );
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 2);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn cmc_refund_and_terminal_results_clear_without_operator_state() -> Result<()> {
        let refunded = Env::new()?;
        update::<_, ()>(
            &refunded.pic,
            refunded.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: refunded.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &refunded.pic,
            refunded.cmc,
            "debug_set_behavior",
            CmcBehavior::Refunded,
        )?;
        refunded.fund()?;
        assert!(
            refunded.state()?.cmc_state.contains("Idle"),
            "explicit refund is terminal and autonomous"
        );

        let terminal = Env::new()?;
        update::<_, ()>(
            &terminal.pic,
            terminal.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: terminal.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &terminal.pic,
            terminal.cmc,
            "debug_set_behavior",
            CmcBehavior::InvalidTransaction,
        )?;
        terminal.fund()?;
        assert!(
            terminal.state()?.cmc_state.contains("Idle"),
            "terminal CMC result does not create admin recovery state"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn subscription_and_cursor_survive_upgrade() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.admit(7, Some("0.01"), 1_000_000_000)?;
        let before = env.state()?;
        env.upgrade_same_debug_wasm()?;
        let after = env.state()?;
        assert_eq!(after.next_block, before.next_block);
        assert_eq!(after.subscriptions, before.subscriptions);
        assert_eq!(
            env.subscription(7)?
                .expect("subscription survives")
                .minimum_e8s,
            1_000_000
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn validated_core_fixture_upgrades_additively() -> Result<()> {
        let env = Env::validated_baseline()?;
        env.poll()?;
        env.admit(7, Some("0.01"), 1_000_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_balance",
            SetBalance {
                account: Account {
                    owner: env.event_horizon,
                    subaccount: None,
                },
                e8s: 100_000_000,
            },
        )?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::CreatedInFuture,
        )?;
        env.fund()?;
        let before: ValidatedCoreDebugState =
            query(&env.pic, env.event_horizon, "debug_state", ())?;
        assert!(before.bootstrapped);
        assert_eq!(before.subscriptions, 1);
        assert!(before.cmc_state.contains("TransferPending"));

        env.upgrade_same_debug_wasm()?;
        let after = env.state()?;
        assert_eq!(after.next_block, before.next_block);
        assert_eq!(after.polling_mode, before.polling_mode);
        assert_eq!(after.subscriptions, before.subscriptions);
        assert!(after.cmc_state.contains("TransferPending"));
        assert_eq!(after.global_subscriptions, 0);
        assert!(!after.pricing.initialized);
        assert_eq!(env.subscription(7)?.unwrap().minimum_e8s, 1_000_000);

        env.price_once()?;
        assert!(
            env.pricing()?.initialized,
            "the first post-upgrade CMC observation initializes pricing"
        );
        update::<_, ()>(&env.pic, env.event_horizon, "debug_start_schedulers", ())?;
        assert_eq!(
            query::<_, u8>(&env.pic, env.event_horizon, "debug_timer_count", ())?,
            3
        );
        update::<_, ()>(&env.pic, env.event_horizon, "debug_start_schedulers", ())?;
        assert_eq!(
            query::<_, u8>(&env.pic, env.event_horizon, "debug_timer_count", ())?,
            3,
            "restarting replaces each timer instead of duplicating it"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn production_wasm_exposes_only_pricing_application_surface() -> Result<()> {
        let pic = PocketIcBuilder::new().with_application_subnet().build();
        let id = pic.create_canister();
        pic.add_cycles(id, 200_000_000_000_000);
        pic.install_canister(
            id,
            wasm(&EVENT_HORIZON_PROD_WASM, "event-horizon", None)?,
            vec![],
            None,
        );
        let pricing: Pricing = query(&pic, id, "get_pricing", ())?;
        assert!(!pricing.initialized);
        let result = pic.query_call(id, Principal::anonymous(), "debug_state", encode_one(())?);
        assert!(
            result.is_err(),
            "production Wasm must not export debug_state"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn frontend_serves_embedded_svg_with_certificate() -> Result<()> {
        let pic = PocketIcBuilder::new().with_application_subnet().build();
        let id = pic.create_canister();
        pic.add_cycles(id, 200_000_000_000_000);
        pic.install_canister(
            id,
            wasm(&FRONTEND_WASM, "event-horizon-frontend", None)?,
            vec![],
            None,
        );
        let svg =
            std::fs::read(workspace_root()?.join("canisters/frontend/public/event-horizon.svg"))?;
        let mut assembled = Vec::new();
        while assembled.len() < svg.len() {
            let headers = if assembled.is_empty() {
                vec![]
            } else {
                vec![("range".to_string(), format!("bytes={}-", assembled.len()))]
            };
            let response: HttpResponse = query(
                &pic,
                id,
                "http_request",
                HttpRequest {
                    method: "GET".to_string(),
                    url: "/event-horizon.svg".to_string(),
                    headers,
                    body: vec![],
                    certificate_version: Some(2),
                },
            )?;
            assert_eq!(response.status_code, 206);
            assert!(response.headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("IC-Certificate") && !value.is_empty()
            }));
            assert!(response.headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("content-range")
                    && value.starts_with(&format!("bytes {}-", assembled.len()))
                    && value.ends_with(&format!("/{}", svg.len()))
            }));
            assert!(!response.body.is_empty());
            assembled.extend(response.body);
        }
        assert_eq!(assembled, svg);

        let pricing_response: HttpResponse = query(
            &pic,
            id,
            "http_request",
            HttpRequest {
                method: "GET".to_string(),
                url: "/pricing.json".to_string(),
                headers: vec![],
                body: vec![],
                certificate_version: Some(2),
            },
        )?;
        assert_eq!(pricing_response.status_code, 404);

        let removed_update = pic.update_call(
            id,
            Principal::anonymous(),
            "http_request_update",
            encode_one(HttpRequest {
                method: "GET".to_string(),
                url: "/pricing.json".to_string(),
                headers: vec![],
                body: vec![],
                certificate_version: Some(2),
            })?,
        );
        assert!(
            removed_update.is_err(),
            "the frontend must not expose the removed pricing update proxy"
        );

        Ok(())
    }
}
