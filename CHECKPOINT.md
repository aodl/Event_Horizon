# Checkpoint 03 — release hardening

Checkpoint 03 builds directly on Checkpoint 02a. It does not broaden the Event Horizon protocol. Its purpose is to make the
implemented design reviewable as a release candidate repository: deterministic builds, stronger failure/upgrade tests,
production-surface auditing, deployment settings and irreversible-controller-removal documentation.

## Protocol carried forward unchanged

- Direct prospective ICP Ledger reader; no Index dependency.
- Fixed-boundary bounded-page polling.
- Live-history only; explicitly archived gaps are logged/skipped and operation continues.
- Exact Jupiter Faucet source authentication.
- Historian-backed permanent admission at 10 ICP qualifying endowment for the exact declaration.
- Optional threshold: `<principal>.<subaccount>` means all incoming transfers; `:<amount>` means inclusive `>=` with an explicit 0.01 ICP floor.
- Per-poll `subscriber -> sorted unique subaccounts` coalescing.
- One best-effort `poke(vec nat8)` attempt per matching subscriber per completed poll.
- No transaction payloads, acknowledgements, retry queue, subscriber accounting or priority.
- Shared cycles-based polling ladder with the agreed hysteresis thresholds.
- Empty production application interface: `service : () -> {}`.
- Certified embedded-assets frontend using the supplied SVG unchanged.

## Important correction made in this checkpoint

Checkpoint 02a's source still used `icrc1_transfer` for the ICP-to-CMC transfer even though its design notes described the
legacy ICP top-up convention. Checkpoint 03 corrects the actual implementation.

The funding lane now uses:

1. `icrc1_balance_of` and `icrc1_fee` to establish usable ICP;
2. the ICP Ledger legacy `transfer` endpoint;
3. the CMC account identifier derived from the CMC principal plus Event Horizon's principal-derived top-up subaccount;
4. the standard numeric `TOP_UP_CANISTER_MEMO`;
5. a persisted deterministic `created_at_time` identity;
6. duplicate recovery to the original block index;
7. `notify_top_up(Event Horizon, block_index)`.

Definite no-debit Ledger failures clear the plan so a later maintenance tick can re-read live balance/fee. Ambiguous transport
outcomes retain the exact transfer identity. CMC `Processing`/transport ambiguity retains the accepted block. Explicit refund
or terminal CMC classification clears autonomously. There is no operator recovery endpoint.

Reserve/cadence decisions and pre-call protection now use the CDK's **liquid** cycles balance, matching the agreed spendability model and avoiding over-counting cycles already reserved by outstanding calls.

## New release-hardening material

- `Cargo.lock` and `package-lock.json` are present.
- `Dockerfile.repro` pins the build environment and produces raw canonical Wasms.
- `tools/scripts/build-release` emits backend/frontend Wasms, hashes and build metadata.
- `tools/scripts/verify-reproducible-artifacts` compares two clean Docker builds byte-for-byte.
- `tools/audit-wasm.py` parses Wasm exports directly and rejects any backend application query/update export.
- `icp.yaml` configures `status_visibility: public`, `log_visibility: public`, and a 4096-byte backend log buffer.
- Controller removal is documented as an explicit manual `--remove-all-controllers` step, never automated.
- Security/CI scaffolding is included with only narrow, documented dependency exceptions inherited from the pinned PocketIC/certification toolchain.
- `cargo run -p xtask -- local-smoke` runs the complete ignored PocketIC suite.

## Expanded integration evidence encoded in tests

The PocketIC source now covers:

- prospective bootstrap;
- 10 ICP admission;
- incomplete/faulted Historian rejection and natural later admission;
- non-Faucet admission rejection;
- unfiltered + thresholded multi-subaccount coalescing;
- subscriber trap isolation;
- archive-gap skip/continue;
- legacy CMC transfer + notify;
- accepted legacy transfer with lost response followed by duplicate-safe recovery without a second spend;
- CMC `Processing` state across an Event Horizon upgrade and later completion;
- explicit CMC refund and terminal-error autonomous clearing;
- subscription and Ledger cursor persistence across upgrade;
- production Wasm rejection of `debug_state`.

## Validation actually executed in the producing environment

Executed successfully:

```text
python3 tools/static-check.py
npm test
python3 -m py_compile tools/audit-wasm.py tools/static-check.py
bash -n tools/scripts/*
TOML/JSON parse checks
```

Results at packaging time:

```text
static repository checks: PASS
frontend memo tests:       4/4 PASS
Python helper syntax:       PASS
shell helper syntax:        PASS
TOML/JSON parsing:          PASS
supplied SVG SHA-256:       edb8090a06441848fbd58c1385c56e1184dc58010d7ffb1704e3fecf43698650
```

The environment used to produce this checkpoint does not contain Rust, Docker or `icp`, so the following source-defined gates
are **not claimed as executed here**:

- Rust compilation / `cargo clippy`;
- PocketIC execution;
- canonical Wasm build/export audit;
- Docker reproducibility comparison;
- cargo-audit/cargo-deny/OSV scan;
- mainnet/local `icp` deployment.

Those are the first gates to run after unzipping. Any compiler/test failure discovered there should be fixed before treating
this checkpoint as a production release candidate.

## Deliberately deferred

- Mainnet observation evidence and calibrated real-world cycles burn.
- Actual Event Horizon mainnet canister ID and Jupiter Faucet `X` alias publication.
- Final controller removal.
- A live frontend Historian endowment-status query. The frontend remains informational and memo-capable; adding browser-side
  Historian querying is presentation functionality and is not needed for backend release correctness.
