# Event Horizon

Event Horizon is a low-latency, best-effort ICP event trigger funded by perpetual Jupiter Faucet endowments.

It deliberately does **not** deliver transaction data. It watches the live ICP Ledger and calls a subscriber's
`poke(vec nat8)` endpoint when watched accounts see relevant transfers. The vector contains only the distinct numbered
subaccounts that matched during that poll. Subscribers remain responsible for their own authoritative Ledger/Index
reconciliation and retain an independent fallback polling cadence. A missed poke therefore affects responsiveness, not correctness.

A subscription may omit its amount threshold to make every incoming transfer relevant, or provide an inclusive minimum
of at least `0.01 ICP`.

The backend exposes no callable production application API and is designed, after a controlled mainnet observation period,
to become immutable by removing all controllers. The separately controlled certified frontend remains informational and upgradeable.

## Checkpoint 03

Checkpoint 03 is the release-hardening checkpoint. In addition to the Checkpoint 02a protocol implementation it now includes:

- corrected CMC funding through the ICP Ledger's **legacy `transfer`** endpoint and standard CMC account/subaccount + top-up memo convention;
- duplicate-safe legacy transfer recovery, including an accepted-transfer/lost-response regression mock;
- expanded PocketIC scenarios for Historian completeness/fault/source admission gates, subscriber traps, CMC processing across upgrades, and stable cursor/subscription restoration;
- a pinned `Cargo.lock` and `package-lock.json`;
- `icp.yaml` with native public backend status/log settings;
- canonical reproducible Docker builds and byte-for-byte double-build verification;
- a Wasm export parser that fails the release if the production backend exposes any application query/update methods;
- deployment, reproducible-build, testing and irreversible controller-removal runbooks;
- basic dependency-security policy and CI scaffolding;
- a one-command PocketIC local smoke workflow.

The production Candid remains exactly:

```candid
service : () -> {}
```

## Subscription memo

Full Jupiter Faucet forms:

```text
X.<compact-subscriber-principal>.<subaccount>
X.<compact-subscriber-principal>.<subaccount>:<minimum-ICP>
```

For example:

```text
X.r5m5ydiaaaaaaaaqanaacai.7
X.r5m5ydiaaaaaaaaqanaacai.7:0.01
```

The thresholded example is exactly 32 bytes.

## First local validation

With Rust 1.94.1 and the `wasm32-unknown-unknown` target installed:

```bash
npm ci
python3 tools/static-check.py
cargo run -p xtask -- check
cargo run -p xtask -- local-smoke
```

Release/reproducibility gates:

```bash
cargo run -p xtask -- release
cargo run -p xtask -- repro
cargo run -p xtask -- security
```

`Dockerfile.repro` is the canonical production build environment.

## Documentation

- [`SPEC.md`](SPEC.md) — normative protocol contract.
- [`docs/subscriber-guide.md`](docs/subscriber-guide.md) — subscriber integration and Index-lag behavior.
- [`docs/operational-backend.md`](docs/operational-backend.md) — autonomous backend flow.
- [`docs/deployment.md`](docs/deployment.md) — controlled mainnet deployment/observation.
- [`docs/reproducible-builds.md`](docs/reproducible-builds.md) — source-to-Wasm verification.
- [`docs/controller-removal.md`](docs/controller-removal.md) — final empty-controller procedure.
- [`CHECKPOINT.md`](CHECKPOINT.md) — exact validation/handover status.
