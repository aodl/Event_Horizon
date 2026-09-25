# Testing and validation

## Which command should I run?

| Goal | Command |
| --- | --- |
| Fast logic tests | `cargo run -p xtask -- unit` |
| Normal code check | `cargo run -p xtask -- check` |
| Integration suite | `cargo run -p xtask -- pocketic` |
| All behavior/source tests | `cargo run -p xtask -- test-all` |
| Everything needed before deployment | `cargo run -p xtask -- validate` |

See the full [developer command guide](../tools/xtask/README.md) for release and security commands.

## Prerequisites

Normal development requires Rust/Cargo with the `wasm32-unknown-unknown` target, Node.js/npm, and Python 3. Run `npm ci` once to install the locked frontend dependencies. PocketIC comes through the Rust test harness and needs no separately managed local replica. Security and release prerequisites are listed in the developer command guide.

## Unit tests

```bash
cargo run -p xtask -- unit
```

This runs the backend library tests and frontend JavaScript tests. It is the fast behavioral loop and deliberately omits format, Clippy, PocketIC, security, and builds. The backend portion can be run directly with:

```bash
cargo test --locked -p event-horizon --lib
```

## Source-quality check

```bash
cargo run -p xtask -- check
```

This is the normal pre-commit gate: repository static invariants, Rust formatting, warnings-denied Clippy, ordinary workspace Rust tests, and frontend Node tests. It does not execute ignored PocketIC scenarios.

## PocketIC integration

```bash
cargo run -p xtask -- pocketic
```

PocketIC scenarios are intentionally `#[ignore]` so ordinary `cargo test --workspace` stays suitable for the normal source gate. The command explicitly selects ignored tests, preserves their output, and serializes them for deterministic canister/failure-state behavior. At the reviewed starting revision there are 37 scenarios; this count may grow.

To run one scenario directly:

```bash
cargo test --locked -p event-horizon-pocketic <test-name> -- --ignored --nocapture --test-threads=1
```

## Full behavioral suite

```bash
cargo run -p xtask -- test-all
```

This runs `check` followed by `pocketic`: all ordinary behavioral/source tests plus the deterministic integration suite, without security scanning or release/Docker builds.

## Security gate

```bash
cargo run -p xtask -- security
```

This runs the repository's unchanged dependency and source policies through `cargo audit`, `cargo deny`, `npm audit --omit=dev`, and OSV Scanner.

## Release builds vs canonical builds

```bash
cargo run -p xtask -- release
cargo run -p xtask -- canonical
```

`release` is a host-toolchain convenience build. `canonical` uses the pinned Docker environment and produces the deployable production artifacts. See [Reproducible builds](reproducible-builds.md).

## Full pre-deployment validation

```bash
cargo run -p xtask -- validate
```

This intentionally expensive command runs `test-all`, security checks, two clean Docker builds proving reproducibility, then a canonical Docker build. The canonical build is last, leaving deployment artifacts under `release-artifacts/`.

## Running narrow tests directly

Raw Cargo and npm commands remain useful while debugging:

```bash
cargo test --locked -p event-horizon --lib pricing
npm test
```

Use `xtask` for acceptance gates so ignored scenarios and repository-specific checks are not accidentally omitted.

## What PocketIC covers

The current integration suite builds purpose-specific mock Wasms and exercises:

- prospective first-install bootstrap;
- exact 10 ICP account and 20 ICP range Historian admission;
- maximum `0-255` expansion, resource reporting, ordinary matching, and upgrade health;
- range threshold, overlap-order, unfiltered overlap, sorted/deduplicated coalescing, and global precedence;
- incomplete/faulted Historian rejection and later natural admission on another Faucet payout;
- non-Faucet admission rejection;
- unfiltered and thresholded watched accounts;
- one poke carrying sorted unique subaccounts per subscriber per poll;
- subscriber trap isolation;
- archive-gap skip-and-continue behavior;
- legacy ICP CMC top-up transfer;
- accepted transfer with lost response and duplicate-safe recovery;
- CMC `Processing` persistence across upgrade and later completion;
- explicit CMC refund and terminal-error autonomous clearing;
- retained-first ordering and CMC Processing/refund/terminal cancellation of planned surplus;
- frozen surplus destination/memo across retained-leg progress, upgrades, ambiguous accepted-transfer recovery, destination replacement, and destination disablement;
- second-gate cancellation when liquid cycles fall after planning;
- new ICP exclusion from an in-flight plan and later classification;
- surplus success, lost response/duplicate recovery, clean reject, insufficient funds, bad fee, future timestamp, `TxTooOld`, and pending-transfer upgrade;
- subscription admission after the original Faucet ICP has flowed through funding/surplus transfers;
- destination-disabled equivalence and zero policy;
- fresh-install `FundingState::Idle`, CMC-only `None`, split-plan `Some(PlannedSurplus)`, and current-Wasm upgrades of every pending funding phase;
- subscription/cursor persistence across upgrade;
- absence of debug application methods in the production Wasm.
