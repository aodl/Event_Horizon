# Event Horizon checkpoint 03: initial local validation

Date: 2026-09-21 UTC. `SPEC.md` remains the normative protocol. This report records executed checks; the baseline `CHECKPOINT.md` was not rewritten.

## Import and environment

- Checkpoint ZIP SHA-256: `25814fd823eb56a27dc8433f39acd794bf2c51e427cdc43ffe99344413eac30b` (matched). The ZIP's `MANIFEST.sha256` covers 77 files; an in-archive hash check found zero mismatches.
- `sha256sum -c MANIFEST.sha256` passed in the untouched extracted checkpoint directory (77 files).
- Untouched import commit and tag: `53b9909`, `checkpoint-03-import`. Work branch: `codex/initial-validation`. The import commit used the local-only `Codex <codex@local.invalid>` identity before the existing Git identity was discovered; later commits use the configured repository identity. The import commit/tag were not rewritten.
- Host: Ubuntu 22.04.5 LTS, Linux `6.8.0-1030-azure`, x86_64. Git 2.34.1; Python 3.10.12 with user-installed `tomli` 2.4.1; Node 24.15.0; npm 11.12.1; Rust 1.94.1; Cargo 1.94.1; rustup 1.29.0; `wasm32-unknown-unknown` installed; `icp` 1.2.0; `dfx` 0.31.0; local `ic-wasm` 0.11.0; Docker 28.0.1; cargo-audit 0.22.2; cargo-deny 0.19.8; OSV Scanner 2.3.8. `DFX_IDENTITY=codex_local` was used for IC tests.
- Fetched the current [ICP Skills catalogue](https://skills.internetcomputer.org/llms.txt), its index, and the current `icp-cli`, `canister-security`, `multi-canister`, `stable-memory`, `cycles-management`, `icrc-ledger`, and `certified-variables` skill files on demand. No skill installer changed repository source.

## Baseline failures before source fixes

| Gate | First observed result |
|---|---|
| `python3 tools/static-check.py` | Failed: Python 3.10 has no `tomllib`. |
| `npm test` | Passed, 4/4 memo tests. |
| `cargo fmt --all -- --check` | Failed across the unformatted checkpoint; it also exposed invalid `''` Rust character literals in PocketIC tests. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Failed before compilation: checkpoint `Cargo.lock` was inconsistent with workspace manifests. |
| `cargo test --workspace` (without `--locked`, to diagnose the lock) | Regenerated the lock, then found missing frontend `candid` dependency and further PocketIC test string literal errors. |
| Initial `cargo run -p xtask -- security` | Found `RUSTSEC-2026-0285` in locked `rustls` 0.23.37. After updating, cargo-deny exposed an incomplete license allowlist. |
| Initial `cargo run -p xtask -- release` | Compiled Wasms but the audit rejected the CDK's reserved timer callback and the IC `debug_print` import as though they were application debug methods. |

The attached handover arrived after the initial import and baseline checks; it was then read in full. The checkpoint ZIP was imported unchanged before any fixes. The original source manifest was verified against ZIP contents after import.

## Changes made

- Regenerated the invalid lockfile, added the frontend's required `candid` manifest dependency, and advanced `rustls` to fixed version 0.23.45 (`rustls-webpki` to 0.103.15). No broad dependency update was performed.
- Fixed PocketIC Rust string literals and the debug Candid export type scope. Scoped debug-only state helpers to the debug feature, corrected a frontend certification API borrow, and formatted the Rust tree to satisfy its existing formatting gate.
- Serialized the PocketIC wrapper to avoid concurrent nested Wasm builds. Added a PocketIC test that reconstructs the supplied 5,463,009-byte SVG from HTTP range responses and checks each chunk's nonempty `IC-Certificate` and content-range headers; it does not independently verify the BLS certificate. Added a mock fault and test proving an unexplained Ledger hole preserves the durable cursor until archive evidence appears.
- Narrowed the Wasm audit exception to the exact CDK internal `timer_executor` export, and kept checks for all application query/update/composite-query exports and named debug methods. The IC `debug_print` system import remains available for exceptional logs.
- Serialized `icp build`'s shared release generation with a local file lock. Before this fix, parallel canister builds left a duplicate `build-info.json` checksum line; a repeat `icp build -e local` produced exactly three manifest entries, all verified.
- Added Python 3.10 `tomli` fallback and narrow ignores for generated release artifacts and Python bytecode. Added the specific licenses required by the existing PocketIC dependency tree to `deny.toml`.
- Recorded the CMC ambiguous-transfer expiry issue in [decision-required-cmc-ambiguous-transfer-expiry.md](decision-required-cmc-ambiguous-transfer-expiry.md); money-moving behavior was not changed.

`SPEC.md`, the public Candid files, the supplied SVG, and the intentionally empty production backend application interface were not changed. No archive traversal, retry/acknowledgement queue, accounting, priority, or administrative endpoint was added.

## Executed gates

| Command | Result |
|---|---|
| `npm ci` | Pass. |
| `python3 tools/static-check.py` | Pass; SVG SHA-256 `edb8090a06441848fbd58c1385c56e1184dc58010d7ffb1704e3fecf43698650`. |
| `npm test` | Pass, 4/4. |
| `python3 -m py_compile tools/audit-wasm.py tools/static-check.py` and `bash -n tools/scripts/*` | Pass. |
| `cargo fmt --all -- --check` | Pass after formatting. |
| `cargo check --locked --workspace --all-targets` | Pass. |
| `cargo metadata --locked --format-version=1` | Pass: 311 packages, 8 workspace members. |
| `cargo test --workspace` | Pass: 21 Rust domain tests; the ignored PocketIC suite was not counted as passing here. |
| `cargo run -p xtask -- check` | Pass: static check, format, Clippy with `-D warnings`, locked workspace tests, and frontend tests. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- pocketic` | Pass, final 12/12. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- local-smoke` | Pass, final 12/12. |
| `DFX_IDENTITY=codex_local cargo test --locked -p event-horizon-pocketic frontend_serves_embedded_svg_with_certificate -- --ignored --nocapture` | Pass. |
| `DFX_IDENTITY=codex_local cargo test --locked -p event-horizon-pocketic unexplained_ledger_hole_preserves_cursor_until_archive_evidence_arrives -- --ignored --nocapture` | Pass. |
| `cargo run -p xtask -- release` | Pass with local `ic-wasm` 0.11.0; both export audits and artifact manifest check pass. |
| `./tools/scripts/docker-build` | Pass on the corrected source with pinned `ic-wasm` 0.9.7; canonical artifact hashes below. An earlier invocation had captured the source before the Wasm-audit correction and failed at its release step. |
| `TMPDIR=/home/codexdev/.cache cargo run -p xtask -- repro` | Pass on retry: `release artifacts match across two clean builds`. Initial attempt failed from a full Docker overlay filesystem, as described below. |
| `DFX_IDENTITY=codex_local icp build -e local` | Pass; repeated after the shared-build lock fix, with three checksum entries verified. This only built; no deployment occurred. |
| `cargo run -p xtask -- security` | Pass after fixes; details below. |

PocketIC cases cover admission/coalescing, Historian completeness/fault recovery, archived gap skip, unexplained-hole retry, CMC success/lost-response duplicate recovery/Processing across upgrade/refund/terminal response, frontend certified SVG ranges, production debug-surface absence, subscriber trap isolation, and subscription/cursor persistence across upgrade. A live interleaving test for activity added during a pinned poll boundary has not been executed.

## Security review

`cargo audit` reports no current vulnerability after the `rustls` update. It reports four permitted unmaintained-package warnings. `cargo tree -i` confirmed `backoff` and `instant` are PocketIC-only; `paste` is Candid proc-macro support; `serde_cbor` is in the certified frontend and PocketIC, not the value-moving backend. `cargo deny check advisories licenses bans sources`, `npm audit --omit=dev`, and `osv-scanner scan -L Cargo.lock -L package-lock.json` pass. OSV's four existing narrow exclusions remain documented in `osv-scanner.toml`. No new advisory was suppressed.

The checked-in CMC and Ledger call types were compared with the current official [ICP Ledger Candid](https://github.com/dfinity/ic/blob/master/rs/ledger_suite/icp/ledger.did) and [CMC Candid](https://github.com/dfinity/ic/blob/master/rs/nns/cmc/cmc.did). The 24-hour ambiguity described in the decision document remains unresolved for controllerless operation.

## Wasm and reproducibility

- Local `ic-wasm` 0.11.0 backend SHA-256: `b9375f2f82186d2486d8447fd50a3571195aa230e5f7f2c3b8d5ecbecda69e79`.
- Local `ic-wasm` 0.11.0 frontend SHA-256: `21b9fdb98648b48912134cb6bd7eca27c7d3f19ac95f80f1cedf16ca7633b78f`.
- Canonical Docker `ic-wasm` 0.9.7 backend SHA-256: `fcd14996cb729c8a9c993b45ecf9120f3f15a1ecefde173e08a468131903efc8`.
- Canonical Docker `ic-wasm` 0.9.7 frontend SHA-256: `a5a029b0fac03749bf726a49904c6f76c95a6d36f1535eb7253343d805dba125`.
- Backend export audit observed only lifecycle/global-timer exports and the exact reserved CDK timer executor update export; no application query/update/composite-query method. Frontend exports certified `http_request`.
- Canonical Docker `build-info.json` records Rust 1.94.1, Cargo 1.94.1, Node 18.20.4, npm 9.2.0, `ic-wasm` 0.9.7, and `SOURCE_DATE_EPOCH=1700000000`. Both canonical export audits and all three checksum entries passed.
- The first `TMPDIR=/home/codexdev/.cache cargo run -p xtask -- repro` attempt failed during the first uncached Docker build at `cargo install ic-wasm` (exit 101). A diagnostic build then failed at `apt-get update` with an invalid-signature error. A Docker container showed its 16 GB overlay filesystem at 100% use; this was a local Docker storage limit. I pruned generated build cache entries older than two hours and confirmed 9.2 GB available before retrying. The retry completed both uncached builds, compared every file under their `release-artifacts` directories with SHA-256, and exited zero: `release artifacts match across two clean builds`.

## Remaining work / limits

- Resolve the [CMC expiry decision](decision-required-cmc-ambiguous-transfer-expiry.md) before claiming safe controllerless operation.
- No ICP mainnet deployment, remote push, Jupiter Faucet `X` alias publication, or controller removal was performed.
