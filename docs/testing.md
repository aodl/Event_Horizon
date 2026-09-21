# Testing strategy

## Pure/domain tests

```bash
cargo run -p xtask -- unit
```

Covers subscription syntax, account derivation, threshold merging and polling hysteresis, plus frontend memo construction.

## PocketIC

```bash
cargo run -p xtask -- pocketic
```

The current integration suite builds purpose-specific mock Wasms and exercises:

- prospective first-install bootstrap;
- exact 10 ICP Historian admission;
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
- subscription/cursor persistence across upgrade;
- absence of debug application methods in the production Wasm.

## Static/release checks

```bash
python3 tools/static-check.py
cargo run -p xtask -- check
cargo run -p xtask -- release
cargo run -p xtask -- repro
cargo run -p xtask -- security
```

The source tree deliberately keeps debug configuration and debug endpoints behind the `debug_api` feature. Canonical release
commands never enable that feature.

## Local smoke

```bash
cargo run -p xtask -- local-smoke
```

This is the same deterministic PocketIC integration suite and is the recommended first command after unzipping the repository.
It requires Rust with `wasm32-unknown-unknown` installed; PocketIC does not require a separately managed local replica.
