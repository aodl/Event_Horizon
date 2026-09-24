# Pre-deployment schema cleanup validation

## Identity and scope

- Starting commit: `419cf8bd5fde877f7f6ac250aa267f1e2992948b`.
- Branch: `codex/predeployment-schema-cleanup`.
- Runtime/test cleanup commit: `fe00d18` (`Remove undeployed funding migration compatibility`).
- Final documentation/manifest commit: the handoff `HEAD`, reported with the archive because a commit cannot contain its own identifier.
- Event Horizon had never been deployed to mainnet or persistent production state, so no migration from development checkpoint schemas is part of the first production contract.
- Production `SURPLUS_CANISTER` remains `None`. No deployment, destination selection, Jupiter Faucet `X` publication, or controller change/removal occurred.

## Compatibility code removed

Production runtime source no longer contains or accesses:

- `CmcState`, `CmcStateValue`, `CMC_MEMORY_ID`, `CMC_STATE`, `with_cmc`, `read_cmc_state`, or any ID-2 initialization;
- `FundingStateV2`, `FundingStateV2Value`, `FUNDING_V2_MEMORY_ID`, `FUNDING_V2_STATE`, or `with_funding_v2`;
- funding `Uninitialized`, first-read migration, or an old stable-cell read/rewrite;
- the incomplete surplus pending variant, `SurplusTransferPendingV2`, or `harden_pending_identity`;
- split compatibility fields whose independently optional destination/memo allowed invalid combinations.

Five historical-Wasm migration tests, four fixture-install helpers, and their legacy Candid adapters were removed from the active PocketIC suite. The fixture binaries and historical reports remain unchanged inert provenance. Active `tests/pocketic/src/lib.rs` contains no fixture `include_bytes!` reference.

## First production schema

Stable IDs remain 0–1 and 3–8. ID 2 is unused and current runtime code never opens it. ID 7 remains funding; ID 8 remains surplus policy. No other stable storage moved.

ID 7 initializes directly to `FundingState::Idle`. Its complete canonical enum is:

```text
Idle
CmcTransferPending { amount_e8s, fee_e8s, created_at_time_nanos, planned_surplus }
CmcNotifyPending { block_index, planned_surplus }
SurplusTransferPending { destination, memo, amount_e8s, fee_e8s, created_at_time_nanos }
```

The shared optional value is:

```text
PlannedSurplus {
    destination: [u8; 32],
    memo: u64,
    amount_e8s: u64,
    fee_e8s: u64,
}
```

`None` is exactly a CMC-only plan. `Some` freezes the complete future surplus account, memo, amount, and fee before the retained Ledger call. The same value crosses retained transfer to CMC notify. Successful minting plus the unchanged 150 T second gate adds the deterministic creation timestamp and persists the sole self-contained surplus pending state before its Ledger call.

## Current-schema coverage

Focused PocketIC tests proved fresh-install Idle; disabled/CMC-only `None`; split `Some(PlannedSurplus)` before retained value moves; unchanged planned value across transfer, CMC notification, and current-Wasm upgrade; complete surplus pending identity across upgrade and duplicate recovery; configuration changes affecting only later plans; and destination disablement not cancelling pending work.

Current-Wasm coverage also preserves the Ledger cursor, polling mode, current pricing, account subscription, small range, global subscription, maximum 0–255 range, all three financial pending phases, and surplus policy. Scheduler startup twice retains exactly three timer slots. No obsolete migration test was replaced merely to preserve count.

## Validation results

- `npm ci` — passed.
- `cargo run -p xtask -- check` — passed: 46 backend unit tests, 12 frontend tests, workspace checks, format, and warnings-denied Clippy.
- `cargo run -p xtask -- pocketic` — 37/37 passed.
- `cargo run -p xtask -- local-smoke` — 37/37 passed.
- `cargo run -p xtask -- security` — passed: zero vulnerabilities; four existing explicitly permitted unmaintained transitive warnings.
- `cargo run -p xtask -- release` — passed, including production export audits.
- `./tools/scripts/docker-build` — passed and verified canonical checksums.
- `cargo run -p xtask -- repro` — passed: release artifacts matched across two independent clean builds.
- `DFX_IDENTITY=codex_local icp build -e local` — passed: `Canisters built successfully`.
- Source manifest generation/verification — passed after this report and final live documentation were staged.

The maximum-range regression consumed `73,444,517` cycles. Stable memory remained `67,174,400` bytes and total memory remained `69,692,549` bytes.

## Static cleanup and artifacts

Searches for `CmcState`, `FundingStateV2`, `SurplusTransferPendingV2`, and `harden_pending_identity` returned no matches in production Rust or active PocketIC source. Searches for the four historical fixture filenames returned no matches in active PocketIC source; remaining references are limited to historical evidence/inventory where applicable.

- Backend Wasm SHA-256: `d5a66897cb914488df53799842a89912f65a0688793b3eb198b064ed7d5c202b`.
- Frontend Wasm SHA-256: `7efa050c60a1ec0a5d19604dc81fb6fae7da5e94d457f92cee5246bca758fe89`.
- Backend audit: 9 total Wasm exports; only application method `canister_query get_pricing`.
- Frontend audit: 7 total Wasm exports; only application method `canister_query http_request`.

Approved polling, subscription, Historian, pricing, CMC, surplus-controller, split, retained-first, receiver, recovery, frontend, and production-surface behavior is unchanged.
