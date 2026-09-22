# Event Horizon post-validation hardening

Date: 2026-09-22 UTC. Starting commit: `b4cfac178d475c8e5267987f6382d30a38624e92` on `codex/initial-validation`. `SPEC.md` remains normative except for the approved narrow guaranteed-response clarification for the value-moving ICP Ledger transfer.

## Behavioral changes

- The legacy ICP Ledger `transfer` used for CMC top-ups now uses `Call::unbounded_wait`. Ledger reads, Historian calls, subscriber pokes, and `notify_top_up` remain bounded.
- `TransferPending` still persists the exact amount, fee, destination derivation, memo, and deterministic `created_at_time`. `TxDuplicate` still recovers the accepted block and `NotifyPending` still persists it for CMC notification.
- `CallErrorExt::is_clean_reject` permits replanning only when the CDK establishes that the call did not execute. Non-clean rejects and decode ambiguity retain the transfer identity.
- `TxTooOld` for an existing identity emits `CMC_TRANSFER_IDENTITY_EXPIRED`, clears that plan, and permits a later maintenance run to re-read live balance and fee and construct a fresh identity.
- Funding maintenance reports whether CMC returned actual `Success`. Only that result causes the scheduler to re-read liquid cycles, update polling mode, and replace the existing poll timer. Refunds, retryable/ambiguous notifications, terminal failures, and an ordinary hourly funding tick do not reschedule polling.
- Added PocketIC regressions for expired-identity autonomous replanning, immediate cadence acceleration after a successful conversion, and the pinned Ledger `chain_length` boundary. The boundary test submits a poll asynchronously, lets its first Ledger response capture the boundary, appends a transaction while the poll is suspended across existing inter-canister work, proves no poke occurs in that poll, and proves the next poll handles it.
- Updated the transfer-expiry decision record with the accepted narrow residual risk. Updated `CHECKPOINT.md` to distinguish the Checkpoint 03 import, initial validation, and this state.
- Preserved the original Checkpoint 03 checksum list at `docs/provenance/checkpoint-03-MANIFEST.sha256`. Added `tools/scripts/source-manifest`; the root manifest now covers current tracked source while excluding itself, `.git`, build outputs, caches, and `release-artifacts`.
- Updated the static validation assertion to require unbounded wait only for legacy `transfer` and bounded wait for the existing Ledger reads.

No production Candid method, subscription grammar, admission amount, trigger behavior, polling threshold, frontend branding, Index dependency, archive lookup, administrative recovery endpoint, journal, or transaction-search subsystem changed.

## Executed evidence

| Command | Result |
|---|---|
| `npm ci` | Pass; dependency tree already current. |
| `cargo run -p xtask -- check` | Initial run correctly failed because the static check still required bounded `transfer`; after updating that assertion, rerun passed. Rust unit result: 21 passed. Frontend result: 4 passed. Clippy, formatting, static checks, workspace tests, and doc tests passed. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- pocketic` | Pass: 15 passed, 0 failed. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- local-smoke` | Pass: 15 passed, 0 failed. |
| `cargo run -p xtask -- security` | Pass. |
| `cargo run -p xtask -- release` | Pass; backend and frontend export audits and all artifact checksum entries passed. |
| `./tools/scripts/docker-build` | Pass; canonical pinned Docker artifacts verified. |
| `TMPDIR=/home/codexdev/.cache cargo run -p xtask -- repro` | Pass: `release artifacts match across two clean builds`. |
| `DFX_IDENTITY=codex_local icp build -e local` | Pass: `Canisters built successfully`; no deployment was performed. |
| `./tools/scripts/source-manifest verify` | Pass after final tracked-source generation; every listed file returned `OK`. |
| `python3 tools/audit-wasm.py release-artifacts/event_horizon.wasm --backend` | Pass: 8 exports. The corresponding frontend audit passed with 7 exports; `sha256sum -c release-artifacts/release-artifacts.sha256` passed all three entries. |

Preliminary compilation used `cargo fmt --all && cargo test --locked -p event-horizon --lib && cargo check --locked --workspace --all-targets`; formatting, 21 unit tests, and the full workspace check passed.

Additional targeted PocketIC commands for each new regression passed before the complete suites:

```text
DFX_IDENTITY=codex_local cargo test --locked -p event-horizon-pocketic expired_transfer_identity_clears_and_later_replans -- --ignored --nocapture --test-threads=1
DFX_IDENTITY=codex_local cargo test --locked -p event-horizon-pocketic successful_cmc_top_up_immediately_accelerates_polling_mode -- --ignored --nocapture --test-threads=1
DFX_IDENTITY=codex_local cargo test --locked -p event-horizon-pocketic transaction_after_pinned_boundary_waits_for_following_poll -- --ignored --nocapture --test-threads=1
```

The current test inventory is 21 backend Rust unit tests, 4 frontend Node tests, and 15 executed PocketIC integration tests. The ignored PocketIC cases shown during ordinary workspace testing are executed by both the dedicated PocketIC and local smoke gates.

## Security and artifacts

`cargo audit` found no current vulnerabilities and reported four existing permitted unmaintained-package warnings: `backoff`, `instant`, `paste`, and `serde_cbor`. `cargo deny` passed advisories, licenses, bans, and sources. `npm audit --omit=dev` found zero vulnerabilities. OSV scanned 311 Cargo packages and zero npm packages, applied the same four documented narrow filters, and found no issues.

Canonical Docker toolchain: Rust/Cargo 1.94.1, Node 18.20.4, npm 9.2.0, `ic-wasm` 0.9.7, `SOURCE_DATE_EPOCH=1700000000`.

- Canonical backend Wasm SHA-256: `13049c27fb41e12a6b4aff6efef77b3693bb0d31fce6b8c8afbea439302cd331`.
- Canonical frontend Wasm SHA-256: `a5a029b0fac03749bf726a49904c6f76c95a6d36f1535eb7253343d805dba125`.
- Canonical `build-info.json` SHA-256: `241d1b191e6fd31067f55fba9f830cbe937b85995eb703acd413461530a452b4`.

## Optional and remaining limitations

The pinned-boundary PocketIC interleaving test was achieved without production hooks.

Independent BLS certificate and witness verification was not added. The official response-verification stack would add a substantial test dependency and PocketIC root-key/raw-response plumbing solely for this optional check. The existing test still reconstructs all 5,463,009 SVG bytes from certified range responses and checks each response's certificate and range headers.

A narrow pathological funding risk remains: after an extraordinary non-clean Ledger failure, a payment could theoretically have been accepted without its block being recoverable. If that identity later returns `TxTooOld`, clearing it can strand that one CMC payment. Unbounded wait removes ordinary timeout/`SYS_UNKNOWN` ambiguity. Eliminating the pathological case requires recovery machinery excluded from Event Horizon v1.

PocketIC cannot make the mock CMC mint cycles. The cadence regression models the real CMC side effect by adding cycles to Event Horizon immediately before the mock returns `Success`, then proves that only `Success` changes and reschedules the mode.

No mainnet deployment, Jupiter Faucet `X` alias publication, controller removal, or remote publication was performed. The ChatGPT review ZIP was repackaged and was not byte-identical to the previously reported Git archive; the canonical source handoff remains the Git commit SHA plus the exact archive SHA-256.
