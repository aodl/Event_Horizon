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
    static ICRC3_LEDGER_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static HISTORIAN_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static CMC_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static SUBSCRIBER_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static EVENT_HORIZON_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static EVENT_HORIZON_PROD_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static FRONTEND_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    const SURPLUS_TRANSFER_MEMO: u64 = u64::from_be_bytes(*b"SURPLUS1");

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

    fn legacy_account_id(owner: Principal, subaccount: [u8; 32]) -> Vec<u8> {
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
    fn account_id(owner: Principal, subaccount: [u8; 32]) -> Account {
        Account {
            owner,
            subaccount: Some(subaccount.to_vec()),
        }
    }
    fn numbered(n: u8) -> [u8; 32] {
        let mut s = [0u8; 32];
        s[31] = n;
        s
    }

    #[derive(CandidType, Deserialize)]
    struct DebugInitArgs {
        observed_ledger: Principal,
        icp_ledger: Principal,
        cmc_canister: Principal,
        historian_canister: Principal,
        faucet_canister: Principal,
        surplus_canister: Option<Principal>,
    }
    #[derive(CandidType, Deserialize)]
    struct InitArgs {
        observed_ledger: Principal,
    }
    #[derive(CandidType, Deserialize)]
    struct SetRoute {
        destination_canister_id: Principal,
        memo: Vec<u8>,
        total_e8s: u64,
    }
    #[derive(CandidType, Deserialize)]
    struct Append {
        from: Account,
        to: Account,
        amount_e8s: u64,
        icrc1_memo: Option<Vec<u8>>,
    }
    #[derive(CandidType, Deserialize)]
    struct DebugProfile {
        symbol: String,
        decimals: u8,
        supports_2xfer: bool,
    }
    #[derive(CandidType, Deserialize)]
    struct DebugCapabilities {
        advertises_icrc1: bool,
        advertises_icrc3: bool,
        advertises_1xfer: bool,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct DebugState {
        admission_bootstrapped: bool,
        admission_next_block: u64,
        observed_bootstrapped: bool,
        observed_next_block: u64,
        polling_mode: PollingMode,
        subscriptions: u64,
        global_subscriptions: u64,
        cmc_state: String,
        surplus_policy: String,
        surplus_destination: Option<Principal>,
        pricing: Pricing,
    }
    #[derive(CandidType, Deserialize)]
    struct DebugCursorArgs {
        admission_bootstrapped: bool,
        admission_next_block: u64,
        observed_bootstrapped: bool,
        observed_next_block: u64,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct InstanceInfo {
        observed_ledger: Principal,
        observed_profile: Option<ObservedProfile>,
        icp_ledger: Principal,
    }
    #[derive(CandidType, Deserialize, Debug)]
    struct ObservedProfile {
        symbol: String,
        decimals: u8,
        supports_icrc2_transfer_from: bool,
    }
    #[derive(CandidType, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
    struct Price {
        account_icp: u64,
        range_icp: u64,
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
        minimum_units: candid::Nat,
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
        observed_ledger: Principal,
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
        fn new_disabled() -> Result<Self> {
            Self::with_event_horizon_wasm_and_surplus(
                wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?,
                200_000_000_000_000,
                false,
            )
        }
        fn new_two_ledgers() -> Result<Self> {
            let env = Self::with_event_horizon_wasm_and_surplus_mode(
                wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?,
                200_000_000_000_000,
                true,
                true,
            )?;
            env.price_once()?;
            Ok(env)
        }
        fn with_event_horizon_wasm(
            event_horizon_wasm: Vec<u8>,
            event_horizon_cycles: u128,
        ) -> Result<Self> {
            Self::with_event_horizon_wasm_and_surplus(
                event_horizon_wasm,
                event_horizon_cycles,
                true,
            )
        }
        fn with_event_horizon_wasm_and_surplus(
            event_horizon_wasm: Vec<u8>,
            event_horizon_cycles: u128,
            surplus_enabled: bool,
        ) -> Result<Self> {
            Self::with_event_horizon_wasm_and_surplus_mode(
                event_horizon_wasm,
                event_horizon_cycles,
                surplus_enabled,
                false,
            )
        }
        fn with_event_horizon_wasm_and_surplus_mode(
            event_horizon_wasm: Vec<u8>,
            event_horizon_cycles: u128,
            surplus_enabled: bool,
            separate_observed: bool,
        ) -> Result<Self> {
            let pic = PocketIcBuilder::new().with_application_subnet().build();
            let ledger = pic.create_canister();
            let observed_ledger = if separate_observed {
                pic.create_canister()
            } else {
                ledger
            };
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
            for id in [ledger, observed_ledger, historian, cmc, subscriber] {
                pic.add_cycles(id, 200_000_000_000_000);
            }
            pic.install_canister(
                ledger,
                wasm(&LEDGER_WASM, "mock-icp-ledger", None)?,
                vec![],
                None,
            );
            if separate_observed {
                pic.install_canister(
                    observed_ledger,
                    wasm(&ICRC3_LEDGER_WASM, "mock-icrc3-ledger", None)?,
                    vec![],
                    None,
                );
                update::<_, ()>(
                    &pic,
                    observed_ledger,
                    "debug_set_profile",
                    DebugProfile {
                        symbol: "IO".into(),
                        decimals: 8,
                        supports_2xfer: true,
                    },
                )?;
            }
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
                    observed_ledger,
                    icp_ledger: ledger,
                    cmc_canister: cmc,
                    historian_canister: historian,
                    faucet_canister: faucet,
                    surplus_canister: surplus_enabled.then_some(subscriber),
                })?,
                None,
            );
            Ok(Self {
                pic,
                ledger,
                observed_ledger,
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
            from: Account,
            to: Account,
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
        fn append_observed(
            &self,
            from: Account,
            to: Account,
            amount: u64,
            memo: Option<Vec<u8>>,
        ) -> Result<u64> {
            update(
                &self.pic,
                self.observed_ledger,
                "debug_append_transfer",
                Append {
                    from,
                    to,
                    amount_e8s: amount,
                    icrc1_memo: memo,
                },
            )
        }
        fn append_observed_transfer_from(
            &self,
            from: Account,
            to: Account,
            amount: u64,
        ) -> Result<u64> {
            update(
                &self.pic,
                self.observed_ledger,
                "debug_append_transfer_from",
                Append {
                    from,
                    to,
                    amount_e8s: amount,
                    icrc1_memo: None,
                },
            )
        }
        fn append_legacy_other(&self, kind: &str) -> Result<u64> {
            update(
                &self.pic,
                self.ledger,
                "debug_append_legacy_other",
                kind.to_string(),
            )
        }
        fn state(&self) -> Result<DebugState> {
            query(&self.pic, self.event_horizon, "debug_state", ())
        }
        fn set_balance(&self, e8s: u64) -> Result<()> {
            update(
                &self.pic,
                self.ledger,
                "debug_set_balance",
                SetBalance {
                    account: Account {
                        owner: self.event_horizon,
                        subaccount: None,
                    },
                    e8s,
                },
            )
        }
        fn raw_balance(&self) -> Result<u64> {
            let value: candid::Nat = query(
                &self.pic,
                self.ledger,
                "icrc1_balance_of",
                Account {
                    owner: self.event_horizon,
                    subaccount: None,
                },
            )?;
            u64::try_from(value.0).map_err(|_| anyhow!("raw ICP balance does not fit u64"))
        }
        fn set_surplus_level(&self, level: u8) -> Result<()> {
            update(
                &self.pic,
                self.event_horizon,
                "debug_set_surplus_level",
                level,
            )
        }
        fn set_surplus_canister(&self, destination: Option<Principal>) -> Result<()> {
            update(
                &self.pic,
                self.event_horizon,
                "debug_set_surplus_canister",
                destination,
            )
        }
        fn set_liquid_cycles_override(&self, value: Option<u128>) -> Result<()> {
            update(
                &self.pic,
                self.event_horizon,
                "debug_set_liquid_cycles_override",
                value,
            )
        }
        fn accepted_with_memo(&self, memo: u64) -> Result<u64> {
            query(
                &self.pic,
                self.ledger,
                "debug_accepted_legacy_transfers_with_memo",
                memo,
            )
        }
        fn accepted_destinations_with_memo(&self, memo: u64) -> Result<Vec<Vec<u8>>> {
            query(
                &self.pic,
                self.ledger,
                "debug_accepted_legacy_destinations_with_memo",
                memo,
            )
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
        fn set_observed_profile_available(&self, value: bool) -> Result<()> {
            update(
                &self.pic,
                self.observed_ledger,
                "debug_set_profile_available",
                value,
            )
        }
        fn set_observed_capabilities(&self, value: DebugCapabilities) -> Result<()> {
            update(
                &self.pic,
                self.observed_ledger,
                "debug_set_capabilities",
                value,
            )
        }
        fn set_observed_profile(&self, value: DebugProfile) -> Result<()> {
            update(&self.pic, self.observed_ledger, "debug_set_profile", value)
        }
        fn set_cursors(&self, args: DebugCursorArgs) -> Result<()> {
            update(&self.pic, self.event_horizon, "debug_set_cursors", args)
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
        fn admit_range(
            &self,
            start: u8,
            end: u8,
            threshold: Option<&str>,
            total: u64,
        ) -> Result<Vec<u8>> {
            let compact = self.subscriber.to_text().replace('-', "");
            let memo = match threshold {
                Some(value) => format!("{compact}.{start}-{end}:{value}"),
                None => format!("{compact}.{start}-{end}"),
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
            // Install-code rate limiting is part of replica behavior; advance beyond the
            // short initial-install window before exercising the upgrade path.
            self.pic.advance_time(std::time::Duration::from_secs(300));
            self.pic.tick();
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
        assert!(env.state()?.observed_bootstrapped);
        env.admit(7, None, 1_000_000_000)?;
        env.admit(8, Some("0.01"), 1_000_000_000)?;
        assert_eq!(
            env.subscription(7)?
                .expect("unfiltered admitted")
                .minimum_units,
            candid::Nat::from(0u8)
        );
        assert_eq!(
            env.subscription(8)?
                .expect("thresholded admitted")
                .minimum_units,
            candid::Nat::from(1_000_000u64)
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
    fn same_wasm_supports_independent_observed_ledger_and_fixed_icp_funding() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.poll()?; // prospective bootstrap for both logs
        let info: InstanceInfo = query(&env.pic, env.event_horizon, "get_instance", ())?;
        assert_eq!(info.observed_ledger, env.observed_ledger);
        assert_eq!(info.icp_ledger, env.ledger);
        env.admit(7, Some("0.00000001"), 1_000_000_000)?;
        let before = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;
        env.append(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            before,
            "unrelated ICP activity is not observed-token activity"
        );
        env.append_observed(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7]
        );

        env.set_balance(100_000_000)?;
        env.fund()?;
        assert_eq!(
            query::<_, u64>(&env.pic, env.ledger, "debug_accepted_legacy_transfers", ())?,
            1,
            "funding calls only the injected ICP ledger"
        );
        assert_eq!(
            query::<_, u64>(
                &env.pic,
                env.observed_ledger,
                "debug_accepted_legacy_transfers",
                ()
            )?,
            0
        );

        let second = env.pic.create_canister();
        env.pic.add_cycles(second, 200_000_000_000_000);
        let bytes = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
        let hash_a = sha2::Sha256::digest(&bytes);
        let hash_b = sha2::Sha256::digest(&bytes);
        assert_eq!(hash_a, hash_b);
        env.pic.install_canister(
            second,
            bytes,
            encode_one(DebugInitArgs {
                observed_ledger: env.ledger,
                icp_ledger: env.ledger,
                cmc_canister: env.cmc,
                historian_canister: env.historian,
                faucet_canister: env.faucet,
                surplus_canister: None,
            })?,
            None,
        );
        let second_info: InstanceInfo = query(&env.pic, second, "get_instance", ())?;
        assert_eq!(second_info.observed_ledger, env.ledger);
        assert_eq!(second_info.icp_ledger, env.ledger);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn icp_instance_uses_one_page_read_for_both_roles() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.append(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(Principal::from_slice(&[8]), [0; 32]),
            1,
            None,
        )?;
        let before = query::<_, u64>(
            &env.pic,
            env.event_horizon,
            "debug_legacy_query_blocks_calls",
            (),
        )?;
        env.poll()?;
        let after = query::<_, u64>(
            &env.pic,
            env.event_horizon,
            "debug_legacy_query_blocks_calls",
            (),
        )?;
        assert_eq!(
            after - before,
            1,
            "one legacy query_blocks page feeds admission and observed processing"
        );
        assert_eq!(
            env.state()?.admission_next_block,
            env.state()?.observed_next_block
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn canonical_icp_mock_does_not_expose_icrc3() -> Result<()> {
        let env = Env::new()?;
        let result = env.pic.query_call(
            env.ledger,
            Principal::anonymous(),
            "icrc3_supported_block_types",
            encode_one(())?,
        );
        assert!(
            result.is_err(),
            "canonical ICP fixture must reject ICRC-3 methods"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn generic_profile_requires_icrc1_icrc3_and_1xfer_but_not_2xfer() -> Result<()> {
        for capabilities in [
            DebugCapabilities {
                advertises_icrc1: false,
                advertises_icrc3: true,
                advertises_1xfer: true,
            },
            DebugCapabilities {
                advertises_icrc1: true,
                advertises_icrc3: false,
                advertises_1xfer: true,
            },
            DebugCapabilities {
                advertises_icrc1: true,
                advertises_icrc3: true,
                advertises_1xfer: false,
            },
        ] {
            let env = Env::new_two_ledgers()?;
            env.set_observed_capabilities(capabilities)?;
            env.poll()?;
            let info: InstanceInfo = query(&env.pic, env.event_horizon, "get_instance", ())?;
            assert!(info.observed_profile.is_none());
            assert!(env.state()?.admission_bootstrapped);
            assert!(!env.state()?.observed_bootstrapped);
        }

        let env = Env::new_two_ledgers()?;
        env.set_observed_profile(DebugProfile {
            symbol: "IO".into(),
            decimals: 18,
            supports_2xfer: false,
        })?;
        env.poll()?;
        let info: InstanceInfo = query(&env.pic, env.event_horizon, "get_instance", ())?;
        let profile = info
            .observed_profile
            .expect("optional 2xfer must not block readiness");
        assert_eq!(profile.symbol, "IO");
        assert_eq!(profile.decimals, 18);
        assert!(!profile.supports_icrc2_transfer_from);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn reserve_protection_does_not_mutate_cursors_or_log_remote_outages() -> Result<()> {
        let wasm = wasm(&EVENT_HORIZON_WASM, "event-horizon", Some("debug_api"))?;
        for separate_observed in [false, true] {
            let env = Env::with_event_horizon_wasm_and_surplus_mode(
                wasm.clone(),
                900_000_000_000,
                false,
                separate_observed,
            )?;
            let before = env.state()?;
            env.poll()?;
            let after = env.state()?;
            assert_eq!(after.admission_bootstrapped, before.admission_bootstrapped);
            assert_eq!(after.admission_next_block, before.admission_next_block);
            assert_eq!(after.observed_bootstrapped, before.observed_bootstrapped);
            assert_eq!(after.observed_next_block, before.observed_next_block);
            let logs = env
                .pic
                .fetch_canister_logs(env.event_horizon, Principal::anonymous())
                .map_err(|error| anyhow!("fetch canister logs: {error:?}"))?;
            let text = logs
                .into_iter()
                .map(|record| String::from_utf8_lossy(&record.content).into_owned())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!text.contains("OBSERVED_LEDGER_UNAVAILABLE"), "{text}");
            assert!(!text.contains("ICP_ADMISSION_LEDGER_UNAVAILABLE"), "{text}");
            assert!(!text.contains("SHARED_ICP_LEDGER_UNAVAILABLE"), "{text}");
        }
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn fresh_profile_failure_preserves_prospective_icp_admission_start() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.set_observed_profile_available(false)?;
        env.poll()?;
        let parked = env.state()?;
        assert!(parked.admission_bootstrapped);
        assert!(!parked.observed_bootstrapped);

        let compact = env.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7:0.00000001").into_bytes();
        env.set_route(memo.clone(), 1_000_000_000)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        env.poll()?;
        assert_eq!(
            env.state()?.admission_next_block,
            parked.admission_next_block
        );

        env.set_observed_profile_available(true)?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_some(),
            "parked payout must be admitted after readiness"
        );
        env.append_observed(
            account_id(Principal::from_slice(&[99]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7]
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn shared_cursor_divergence_fails_closed_without_rewind_or_fetch() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let before_calls = query::<_, u64>(
            &env.pic,
            env.event_horizon,
            "debug_legacy_query_blocks_calls",
            (),
        )?;
        env.set_cursors(DebugCursorArgs {
            admission_bootstrapped: true,
            admission_next_block: 3,
            observed_bootstrapped: true,
            observed_next_block: 4,
        })?;
        env.poll()?;
        let state = env.state()?;
        assert_eq!(
            (state.admission_next_block, state.observed_next_block),
            (3, 4)
        );
        assert_eq!(
            query::<_, u64>(
                &env.pic,
                env.event_horizon,
                "debug_legacy_query_blocks_calls",
                ()
            )?,
            before_calls
        );
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            0
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn shared_icp_scan_applies_admission_in_legacy_block_order() -> Result<()> {
        let earlier_transfer = Env::new()?;
        earlier_transfer.poll()?;
        let compact = earlier_transfer.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7").into_bytes();
        earlier_transfer.set_route(memo.clone(), 1_000_000_000)?;
        earlier_transfer.append(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(earlier_transfer.subscriber, numbered(7)),
            1,
            None,
        )?;
        earlier_transfer.append(
            account_id(earlier_transfer.faucet, [0; 32]),
            account_id(earlier_transfer.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        earlier_transfer.poll()?;
        for _ in 0..5 {
            earlier_transfer.pic.tick();
        }
        assert!(earlier_transfer.subscription(7)?.is_some());
        assert_eq!(
            query::<_, u64>(
                &earlier_transfer.pic,
                earlier_transfer.subscriber,
                "debug_pokes",
                ()
            )?,
            0,
            "a transfer before its admission must not match retroactively"
        );

        let later_transfer = Env::new()?;
        later_transfer.poll()?;
        let compact = later_transfer.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7").into_bytes();
        later_transfer.set_route(memo.clone(), 1_000_000_000)?;
        later_transfer.append(
            account_id(later_transfer.faucet, [0; 32]),
            account_id(later_transfer.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        later_transfer.append(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(later_transfer.subscriber, numbered(7)),
            1,
            None,
        )?;
        later_transfer.poll()?;
        for _ in 0..5 {
            later_transfer.pic.tick();
        }
        assert_eq!(
            query::<_, Vec<u8>>(
                &later_transfer.pic,
                later_transfer.subscriber,
                "debug_last_subaccounts",
                ()
            )?,
            vec![7]
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn transfer_from_global_other_and_malformed_retry_semantics() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.poll()?;
        env.admit(7, None, 1_000_000_000)?;
        env.admit_global(10_000_000_000)?;
        env.append_observed_transfer_from(
            account_id(Principal::from_slice(&[9]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7]
        );
        update::<_, u64>(
            &env.pic,
            env.observed_ledger,
            "debug_append_other",
            "3approve".to_string(),
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?.is_empty()
        );
        let before = env.state()?.observed_next_block;
        update::<_, u64>(
            &env.pic,
            env.observed_ledger,
            "debug_append_malformed_transfer",
            (),
        )?;
        env.poll()?;
        assert_eq!(env.state()?.observed_next_block, before);
        env.poll()?;
        assert_eq!(env.state()?.observed_next_block, before);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn live_page_cursor_is_atomic_across_a_late_malformed_block() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.poll()?;
        env.admit(7, None, 1_000_000_000)?;
        let source = account_id(Principal::from_slice(&[91]), [0; 32]);
        env.append_observed(
            source.clone(),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        update::<_, u64>(
            &env.pic,
            env.observed_ledger,
            "debug_append_malformed_transfer",
            (),
        )?;
        let before = env.state()?.observed_next_block;
        let pokes = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;
        env.poll()?;
        assert_eq!(env.state()?.observed_next_block, before);
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            pokes
        );

        update::<_, ()>(
            &env.pic,
            env.observed_ledger,
            "debug_repair_last_transfer",
            Append {
                from: source,
                to: account_id(Principal::from_slice(&[92]), [0; 32]),
                amount_e8s: 1,
                icrc1_memo: None,
            },
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            pokes + 1
        );
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7]
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn archive_only_progress_does_not_poke_a_global_subscriber() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.poll()?;
        env.admit_global(10_000_000_000)?;
        update::<_, u64>(
            &env.pic,
            env.observed_ledger,
            "debug_append_other",
            "3approve".to_string(),
        )?;
        update::<_, ()>(
            &env.pic,
            env.observed_ledger,
            "debug_set_first_local_block",
            1u64,
        )?;
        let pokes = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            pokes
        );
        assert_eq!(env.state()?.observed_next_block, 1);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn distinct_ledger_admission_is_not_retroactive_within_a_poll() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.poll()?;
        let compact = env.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7").into_bytes();
        env.append_observed(
            account_id(Principal::from_slice(&[93]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.set_route(memo.clone(), 1_000_000_000)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert!(env.subscription(7)?.is_some());
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            0
        );

        env.append_observed(
            account_id(Principal::from_slice(&[94]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            1
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn distinct_stream_failures_do_not_suppress_the_other_stream() -> Result<()> {
        let env = Env::new_two_ledgers()?;
        env.poll()?;

        let compact = env.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7").into_bytes();
        env.set_route(memo.clone(), 1_000_000_000)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        update::<_, u64>(
            &env.pic,
            env.observed_ledger,
            "debug_append_malformed_transfer",
            (),
        )?;
        env.poll()?;
        assert!(
            env.subscription(7)?.is_some(),
            "admission must survive observed failure"
        );

        update::<_, ()>(
            &env.pic,
            env.observed_ledger,
            "debug_repair_last_transfer",
            Append {
                from: account_id(Principal::from_slice(&[96]), [0; 32]),
                to: account_id(Principal::from_slice(&[97]), [0; 32]),
                amount_e8s: 1,
                icrc1_memo: None,
            },
        )?;

        update::<_, u64>(&env.pic, env.ledger, "debug_append_malformed_transfer", ())?;
        env.append_observed(
            account_id(Principal::from_slice(&[95]), [0; 32]),
            account_id(env.subscriber, numbered(7)),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick()
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            1
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
    fn range_admission_uses_exact_route_source_and_twenty_icp_tier() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let compact = env.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.7-18").into_bytes();

        env.admit_range(7, 18, None, 1_999_999_999)?;
        assert_eq!(env.state()?.subscriptions, 0, "range tier rejects N-1 e8s");

        env.set_route(memo.clone(), 2_000_000_000)?;
        env.append(
            account_id(Principal::from_slice(&[99]), [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo.clone()),
        )?;
        env.poll()?;
        assert_eq!(
            env.state()?.subscriptions,
            0,
            "non-Faucet source cannot admit"
        );

        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            1,
            Some(memo),
        )?;
        env.poll()?;
        assert_eq!(
            env.state()?.subscriptions,
            12,
            "later cumulative payout admits"
        );

        for invalid in [format!("{compact}.10-9"), format!("{compact}.7-7")] {
            let invalid = invalid.into_bytes();
            env.set_route(invalid.clone(), 10_000_000_000)?;
            env.append(
                account_id(env.faucet, [0; 32]),
                account_id(env.event_horizon, [0; 32]),
                1,
                Some(invalid),
            )?;
            env.poll()?;
        }
        assert_eq!(
            env.state()?.subscriptions,
            12,
            "reverse and degenerate ranges are never normalized or admitted"
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn maximum_range_expands_256_entries_and_remains_healthy() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        let status_before = env
            .pic
            .canister_status(env.event_horizon, None)
            .map_err(|error| anyhow!("status before maximum range: {error:?}"))?;
        let cycles_before = env.pic.cycle_balance(env.event_horizon);

        env.admit_range(0, 255, None, 2_000_000_000)?;
        let cycles_after = env.pic.cycle_balance(env.event_horizon);
        let status_after = env
            .pic
            .canister_status(env.event_horizon, None)
            .map_err(|error| anyhow!("status after maximum range: {error:?}"))?;
        assert_eq!(env.state()?.subscriptions, 256);
        for subaccount in 0..=255u8 {
            let subscription = env
                .subscription(subaccount)?
                .expect("expanded account exists");
            assert_eq!(subscription.numbered_subaccount, subaccount);
            assert_eq!(subscription.minimum_units, candid::Nat::from(0u8));
        }
        println!(
            "maximum_range_resource cycles_consumed={} stable_memory_before={} stable_memory_after={} total_memory_before={} total_memory_after={}",
            cycles_before.saturating_sub(cycles_after),
            status_before.memory_metrics.stable_memory_size,
            status_after.memory_metrics.stable_memory_size,
            status_before.memory_size,
            status_after.memory_size,
        );

        let source = account_id(Principal::from_slice(&[9]), [0; 32]);
        for subaccount in [0, 128, 255] {
            env.append(
                source.clone(),
                account_id(env.subscriber, numbered(subaccount)),
                1,
                None,
            )?;
        }
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![0, 128, 255]
        );

        env.upgrade_same_debug_wasm()?;
        assert_eq!(env.state()?.subscriptions, 256);
        assert!(env.subscription(255)?.is_some());
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn range_matching_overlap_coalescing_and_global_precedence_hold() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.admit_global(10_000_000_000)?;
        env.admit_range(7, 18, Some("0.1"), 2_000_000_000)?;
        let source = account_id(Principal::from_slice(&[9]), [0; 32]);
        let before = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;

        for (subaccount, amount) in [
            (6, 100_000_000),
            (7, 9_999_999),
            (11, 10_000_000),
            (18, 100_000_000),
            (19, 100_000_000),
        ] {
            env.append(
                source.clone(),
                account_id(env.subscriber, numbered(subaccount)),
                amount,
                None,
            )?;
        }
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            before + 1
        );
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![11, 18],
            "only actual qualifying range accounts are sent"
        );

        env.append(
            source.clone(),
            account_id(Principal::from_slice(&[88]), [0; 32]),
            1,
            None,
        )?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?.is_empty(),
            "unrelated activity retains the one empty global hint"
        );

        env.admit(7, Some("1"), 1_000_000_000)?;
        assert_eq!(
            env.subscription(7)?.unwrap().minimum_units,
            candid::Nat::from(10_000_000u64)
        );
        env.admit(7, None, 1_000_000_000)?;
        assert_eq!(
            env.subscription(7)?.unwrap().minimum_units,
            candid::Nat::from(0u8)
        );
        for subaccount in [18, 7, 11, 7] {
            env.append(
                source.clone(),
                account_id(env.subscriber, numbered(subaccount)),
                1,
                None,
            )?;
        }
        let pokes_before = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert_eq!(
            query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
            pokes_before + 1
        );
        assert_eq!(
            query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?,
            vec![7],
            "only account 7 is unfiltered; thresholded range accounts remain filtered"
        );

        let reverse = Env::new()?;
        reverse.poll()?;
        reverse.admit_range(5, 10, Some("0.1"), 2_000_000_000)?;
        reverse.admit(7, Some("1"), 1_000_000_000)?;
        assert_eq!(
            reverse.subscription(7)?.unwrap().minimum_units,
            candid::Nat::from(10_000_000u64)
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn icp_mint_burn_and_approve_are_global_only_activity() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.admit(7, None, 1_000_000_000)?;
        env.admit_global(10_000_000_000)?;
        for kind in ["mint", "burn", "approve"] {
            let before = query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?;
            env.append_legacy_other(kind)?;
            env.poll()?;
            for _ in 0..5 {
                env.pic.tick();
            }
            assert_eq!(
                query::<_, u64>(&env.pic, env.subscriber, "debug_pokes", ())?,
                before + 1,
                "{kind} must count as one global activity"
            );
            assert!(
                query::<_, Vec<u8>>(&env.pic, env.subscriber, "debug_last_subaccounts", ())?
                    .is_empty(),
                "{kind} must not match a specific watched account"
            );
        }
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
                range_icp: 20,
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
                range_icp: 10,
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
        let after_failed = env.state()?.observed_next_block;
        update::<_, ()>(&env.pic, env.subscriber, "debug_set_trap", false)?;
        env.append(source, account_id(env.subscriber, numbered(7)), 1, None)?;
        env.poll()?;
        for _ in 0..5 {
            env.pic.tick();
        }
        assert!(
            env.state()?.observed_next_block > after_failed,
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
        assert_eq!(env.state()?.observed_next_block, 5);
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
            env.state()?.observed_next_block,
            0,
            "unexplained hole must be retried"
        );

        update::<_, ()>(&env.pic, env.ledger, "debug_suppress_archive_info", false)?;
        env.poll()?;
        assert_eq!(env.state()?.observed_next_block, 5);
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
    fn disabled_destination_preserves_single_leg_funding_and_resets_policy() -> Result<()> {
        let env = Env::new_disabled()?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
        assert_eq!(
            query::<_, u64>(&env.pic, env.ledger, "debug_accepted_legacy_transfers", ())?,
            1
        );
        let state = env.state()?;
        assert_eq!(state.surplus_destination, None);
        assert!(state.surplus_policy.contains("initialized: false"));
        assert!(state.surplus_policy.contains("diversion_level: 0"));
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn fresh_install_funding_is_idle_and_cmc_only_plan_has_no_surplus() -> Result<()> {
        let env = Env::new_disabled()?;
        assert!(env.state()?.cmc_state.contains("Idle"));
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        let pending = env.state()?.cmc_state;
        assert!(pending.contains("CmcNotifyPending"));
        assert!(pending.contains("planned_surplus: None"));
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn retained_top_up_completes_before_surplus_can_leave() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("CmcNotifyPending"));
        assert!(env
            .state()?
            .cmc_state
            .contains("planned_surplus: Some(PlannedSurplus"));
        assert!(env.state()?.cmc_state.contains("amount_e8s: 94981000"));
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);

        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(env.raw_balance()?, 0);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn cycles_falling_after_planning_cancel_surplus_and_reclassify_later() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("CmcNotifyPending"));

        env.set_liquid_cycles_override(Some(149_000_000_000_000))?;
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(env.raw_balance()?, 94_991_000);

        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
        assert_eq!(env.raw_balance()?, 0);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn new_icp_arriving_during_plan_is_not_incorporated() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert_eq!(env.raw_balance()?, 94_991_000);
        env.set_balance(144_991_000)?;

        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert_eq!(env.raw_balance()?, 50_000_000);
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);

        env.fund()?;
        assert_eq!(env.raw_balance()?, 0);
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 2);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn surplus_lost_response_recovers_duplicate_across_upgrade() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::DropResponseAfterAccept,
        )?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        let pending = env.state()?.cmc_state;
        assert!(pending.contains("SurplusTransferPending"));

        env.upgrade_same_debug_wasm()?;
        assert_eq!(env.state()?.cmc_state, pending);
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        assert!(env.state()?.cmc_state.contains("Idle"));
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn uncertain_surplus_transfer_stays_bound_to_original_destination() -> Result<()> {
        let env = Env::new()?;
        let destination_a = legacy_account_id(env.subscriber, [0; 32]);
        let destination_b = legacy_account_id(env.historian, [0; 32]);
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::DropResponseAfterAccept,
        )?;
        env.fund()?;
        assert_eq!(
            env.accepted_destinations_with_memo(SURPLUS_TRANSFER_MEMO)?,
            vec![destination_a.clone()]
        );
        let pending = env.state()?.cmc_state;
        assert!(pending.contains("SurplusTransferPending"));
        assert!(pending.contains(&format!("memo: {SURPLUS_TRANSFER_MEMO}")));

        env.upgrade_same_debug_wasm()?;
        env.set_surplus_canister(Some(env.historian))?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(
            env.accepted_destinations_with_memo(SURPLUS_TRANSFER_MEMO)?,
            vec![destination_a]
        );

        env.set_balance(100_000_000)?;
        env.fund()?;
        assert_eq!(
            env.accepted_destinations_with_memo(SURPLUS_TRANSFER_MEMO)?,
            vec![legacy_account_id(env.subscriber, [0; 32]), destination_b]
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn retained_plan_carries_original_surplus_identity_across_config_change() -> Result<()> {
        let env = Env::new()?;
        let destination_a = legacy_account_id(env.subscriber, [0; 32]);
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        let pending = env.state()?.cmc_state;
        assert!(pending.contains("CmcNotifyPending"));
        assert!(pending.contains("planned_surplus: Some(PlannedSurplus"));
        assert!(pending.contains(&format!("memo: {SURPLUS_TRANSFER_MEMO}")));

        env.set_surplus_canister(Some(env.historian))?;
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert_eq!(
            env.accepted_destinations_with_memo(SURPLUS_TRANSFER_MEMO)?,
            vec![destination_a]
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn disabling_destination_does_not_cancel_pending_surplus_identity() -> Result<()> {
        let env = Env::new()?;
        let destination_a = legacy_account_id(env.subscriber, [0; 32]);
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::DropResponseAfterAccept,
        )?;
        env.fund()?;
        env.set_surplus_canister(None)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(
            env.accepted_destinations_with_memo(SURPLUS_TRANSFER_MEMO)?,
            vec![destination_a]
        );
        assert!(env.state()?.cmc_state.contains("Idle"));

        env.set_balance(100_000_000)?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn surplus_definite_failures_clear_for_conservative_replan() -> Result<()> {
        for behavior in [LegacyBehavior::InsufficientFunds, LegacyBehavior::BadFee] {
            let env = Env::new()?;
            env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
            env.set_surplus_level(19)?;
            env.set_balance(100_000_000)?;
            update::<_, ()>(
                &env.pic,
                env.ledger,
                "debug_set_surplus_legacy_behavior",
                behavior,
            )?;
            env.fund()?;
            assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
            assert!(env.state()?.cmc_state.contains("Idle"));
            assert_eq!(env.raw_balance()?, 94_991_000);
        }
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn missing_ledger_clean_reject_cancels_surplus_for_replan() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.cmc,
            "debug_set_behavior",
            CmcBehavior::Processing,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("CmcNotifyPending"));

        update::<_, ()>(
            &env.pic,
            env.event_horizon,
            "debug_set_ledger_canister",
            Principal::from_slice(&[99]),
        )?;
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("Idle"));
        update::<_, ()>(
            &env.pic,
            env.event_horizon,
            "debug_set_ledger_canister",
            env.ledger,
        )?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
        assert_eq!(env.raw_balance()?, 94_991_000);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn surplus_future_timestamp_retries_and_too_old_expires() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::CreatedInFuture,
        )?;
        env.fund()?;
        let pending = env.state()?.cmc_state;
        assert!(pending.contains("SurplusTransferPending"));

        env.fund()?;
        assert_eq!(env.state()?.cmc_state, pending);
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_surplus_legacy_behavior",
            LegacyBehavior::TooOld,
        )?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn lost_legacy_transfer_response_reuses_identity_without_second_spend() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
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
        assert!(env
            .state()?
            .cmc_state
            .contains("planned_surplus: Some(PlannedSurplus"));
        assert!(env.state()?.cmc_state.contains("amount_e8s: 94981000"));
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(
            env.accepted_with_memo(1_347_768_404)?,
            1,
            "duplicate recovery must not spend twice"
        );
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 1);
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn expired_transfer_identity_clears_and_later_replans() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
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
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);

        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(1_347_768_404)?, 1);
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 1);
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
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
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
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
        let pending = env.state()?.cmc_state;
        assert!(pending.contains("NotifyPending"));
        assert!(pending.contains("planned_surplus: Some(PlannedSurplus"));
        assert!(pending.contains("amount_e8s: 94981000"));
        env.upgrade_same_debug_wasm()?;
        assert_eq!(env.state()?.cmc_state, pending);
        update::<_, ()>(&env.pic, env.cmc, "debug_set_behavior", CmcBehavior::Ok)?;
        env.fund()?;
        assert!(env.state()?.cmc_state.contains("Idle"));
        assert_eq!(query::<_, u64>(&env.pic, env.cmc, "debug_calls", ())?, 2);
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn retained_transfer_plan_with_surplus_survives_upgrade() -> Result<()> {
        let env = Env::new()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;
        env.set_balance(100_000_000)?;
        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::CreatedInFuture,
        )?;
        env.fund()?;
        let before = env.state()?.cmc_state;
        let policy_before = env.state()?.surplus_policy;
        assert!(before.contains("CmcTransferPending"));
        assert!(before.contains("planned_surplus: Some(PlannedSurplus"));
        assert!(before.contains("amount_e8s: 94981000"));
        env.upgrade_same_debug_wasm()?;
        assert_eq!(env.state()?.cmc_state, before);
        assert_eq!(env.state()?.surplus_policy, policy_before);

        update::<_, ()>(
            &env.pic,
            env.ledger,
            "debug_set_legacy_behavior",
            LegacyBehavior::Normal,
        )?;
        env.fund()?;
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);
        assert!(env.state()?.cmc_state.contains("Idle"));
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn cmc_refund_and_terminal_results_clear_without_operator_state() -> Result<()> {
        let refunded = Env::new()?;
        refunded.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        refunded.set_surplus_level(19)?;
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
        assert_eq!(refunded.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);

        let terminal = Env::new()?;
        terminal.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        terminal.set_surplus_level(19)?;
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
        assert_eq!(terminal.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 0);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn subscription_and_cursor_survive_upgrade() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.admit(7, Some("0.01"), 1_000_000_000)?;
        env.admit_range(8, 9, None, 10_000_000_000)?;
        env.admit_global(10_000_000_000)?;
        let before = env.state()?;
        let pricing_before = env.pricing()?;
        env.upgrade_same_debug_wasm()?;
        let after = env.state()?;
        assert_eq!(after.observed_next_block, before.observed_next_block);
        assert_eq!(after.polling_mode, before.polling_mode);
        assert_eq!(after.subscriptions, before.subscriptions);
        assert_eq!(after.global_subscriptions, before.global_subscriptions);
        assert_eq!(env.pricing()?, pricing_before);
        assert_eq!(
            env.subscription(7)?
                .expect("subscription survives")
                .minimum_units,
            candid::Nat::from(1_000_000u64)
        );
        assert!(env.subscription(8)?.is_some());
        assert!(env.subscription(9)?.is_some());
        assert!(env.global_subscription()?);
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn current_schema_scheduler_start_is_idempotent() -> Result<()> {
        let env = Env::new()?;
        update::<_, ()>(&env.pic, env.event_horizon, "debug_start_schedulers", ())?;
        assert_eq!(
            query::<_, u8>(&env.pic, env.event_horizon, "debug_timer_count", ())?,
            3
        );
        update::<_, ()>(&env.pic, env.event_horizon, "debug_start_schedulers", ())?;
        assert_eq!(
            query::<_, u8>(&env.pic, env.event_horizon, "debug_timer_count", ())?,
            3
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn diversion_does_not_affect_later_subscription_admission() -> Result<()> {
        let env = Env::new()?;
        env.poll()?;
        env.set_liquid_cycles_override(Some(200_000_000_000_000))?;
        env.set_surplus_level(19)?;

        let compact = env.subscriber.to_text().replace('-', "");
        let memo = format!("{compact}.17:0.01").into_bytes();
        env.set_route(memo.clone(), 1_000_000_000)?;
        env.append(
            account_id(env.faucet, [0; 32]),
            account_id(env.event_horizon, [0; 32]),
            100_000_000,
            Some(memo),
        )?;
        env.set_balance(100_000_000)?;
        env.fund()?;
        assert_eq!(env.raw_balance()?, 0);
        assert_eq!(env.accepted_with_memo(SURPLUS_TRANSFER_MEMO)?, 1);

        env.poll()?;
        assert_eq!(
            env.subscription(17)?.unwrap().minimum_units,
            candid::Nat::from(1_000_000u64)
        );
        Ok(())
    }

    #[test]
    #[ignore = "builds wasm and runs PocketIC"]
    fn production_wasm_exposes_only_instance_and_pricing_queries() -> Result<()> {
        let pic = PocketIcBuilder::new().with_application_subnet().build();
        let id = pic.create_canister();
        pic.add_cycles(id, 200_000_000_000_000);
        pic.install_canister(
            id,
            wasm(&EVENT_HORIZON_PROD_WASM, "event-horizon", None)?,
            encode_one(InitArgs {
                observed_ledger: id,
            })?,
            None,
        );
        let pricing: Pricing = query(&pic, id, "get_pricing", ())?;
        assert!(!pricing.initialized);
        pic.query_call(id, Principal::anonymous(), "get_instance", encode_one(())?)
            .map_err(|e| anyhow!("get_instance: {e:?}"))?;
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
