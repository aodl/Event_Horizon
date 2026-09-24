# Surplus transfer identity hardening

## Scope and commits

- Starting commit: `4d21a35822d0b9ce04fd097a5e99abb39ccb23d3`.
- Branch: `codex/surplus-diversion`.
- Implementation commit: `a8986a0` (`Freeze surplus transfer identity in funding state`).
- Final evidence/manifest commit: the handoff `HEAD`, reported with the archive because a commit cannot contain its own identifier.
- Production `SURPLUS_CANISTER`: `None`.
- No policy, split arithmetic, subscription, pricing, polling, production API, or treasury-economics change was made.
- No mainnet deployment, Jupiter Faucet `X` publication, or controller change/removal was performed.

## Persisted identity

Every newly created split plan now freezes the configured canister's default ICP account identifier and `SURPLUS_TRANSFER_MEMO` before the retained CMC transfer begins. `CmcTransferPending` and `CmcNotifyPending` persist:

- retained transfer identity / CMC block as before;
- `planned_surplus_e8s: u64`;
- `planned_surplus_fee_e8s: u64`;
- `planned_surplus_destination: Option<[u8; 32]>`;
- `planned_surplus_memo: Option<u64>`.

For a new nonzero surplus plan both optional identity fields are `Some`. Pre-surplus old CMC migrations have zero surplus and both fields `None`; no destination is fabricated.

After CMC mint success and the unchanged post-mint 150 T gate, `SurplusTransferPendingV2` is self-contained:

```text
destination: [u8; 32]
memo: u64
amount_e8s: u64
fee_e8s: u64
created_at_time_nanos: u64
```

`resume_surplus_transfer` constructs the Ledger argument only from those stable fields. It does not read the current surplus canister or the memo constant. A destination change or disablement therefore affects future Idle plans only. It cannot redirect or cancel an existing plan. The configured Ledger canister remains the existing trusted runtime endpoint; this pass changes only the transferred transaction's destination/memo identity.

## Stable compatibility and migration

Stable memory IDs remain unchanged: ID 7 is still the sole `FundingStateV2` authority and ID 8 remains the surplus controller. No cell or public method was added.

The exact `304b26e9e6facc6f8812e61c56f98e712e05f803` fixture still migrates:

- old Idle → new Idle;
- old TransferPending → identical retained identity, zero surplus, absent surplus destination/memo;
- old NotifyPending → identical block, zero surplus, absent surplus destination/memo.

The undeployed `4d21a35822d0b9ce04fd097a5e99abb39ccb23d3` FundingStateV2 record remains Candid-decodable through additive optional fields. Its exact tracked debug fixture is `tests/fixtures/event_horizon_surplus_4d21a35_debug.wasm`, SHA-256 `0a29874e1ebf4d6c387e12532ceb11680a48413a0d7f023dde7cad905a3d88a8`. During post-upgrade, before timers or debug calls resume, legacy surplus-bearing pending state is hardened with the stable pre-upgrade debug destination and fixed memo. The retained legacy surplus-transfer variant exists only to decode that fixture and is immediately rewritten as self-contained V2 when its old destination remains available.

PocketIC proved an accepted transfer with a lost response at destination A upgrades from this exact fixture, is hardened to A, then recovers as the Ledger duplicate after debug configuration changes to B. No transfer to B occurs. Current-Wasm tests separately proved the same A binding across upgrade/configuration change, a change while CMC notification is pending, and destination disablement while the surplus transfer is pending. A later fresh Idle plan uses B, while disabled fresh plans remain CMC-only.

## Validation

- `npm ci` — passed.
- `cargo run -p xtask -- check` — passed: 46 backend Rust unit tests, 12 frontend tests, and all workspace tests.
- `cargo run -p xtask -- pocketic` — 40/40 passed.
- `cargo run -p xtask -- local-smoke` — 40/40 passed.
- Exact `304b26e…` old-CMC migration — passed for Idle, transfer pending, and notify pending.
- Exact `4d21a35…` uncertain accepted surplus migration — passed.
- `cargo run -p xtask -- security` — passed: zero vulnerabilities; the same four explicitly permitted unmaintained transitive warnings.
- `cargo run -p xtask -- release` — passed, including production export audits.
- `./tools/scripts/docker-build` — passed and verified canonical artifact checksums.
- `cargo run -p xtask -- repro` — passed after reclaiming unused Docker build cache: release artifacts matched across two independent clean builds. An earlier attempt failed during image package installation because the host Docker cache had exhausted available space; no project code ran in that failed attempt.
- `DFX_IDENTITY=codex_local icp build -e local` — passed: `Canisters built successfully`.
- Source manifest generation/verification — performed after this report and final documentation.

The maximum-range regression consumed `72,119,501` cycles. Stable memory remained `75,563,008` bytes; total memory remained `78,099,369` bytes. Production export surfaces remain backend `get_pricing` only and frontend `http_request` only.

## Canonical artifacts and limitations

- Backend Wasm SHA-256: `428cea8601b4c0b4c73bd3d98120e1170d044115d43aa6200397337ec5d37416`.
- Frontend Wasm SHA-256: `7efa050c60a1ec0a5d19604dc81fb6fae7da5e94d457f92cee5246bca758fe89`.
- Backend audit: 9 total Wasm exports; only application method `canister_query get_pricing`.
- Frontend audit: 7 total Wasm exports; only application method `canister_query http_request`.

The production destination remains intentionally disabled. The compatibility hardening of an old `4d21…` surplus-bearing plan necessarily uses that undeployed Wasm's pre-upgrade debug configuration because the old encoding did not contain the missing destination/memo; production never deployed that intermediate implementation. All newly created plans are self-contained before their first value-moving call.
