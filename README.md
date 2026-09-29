# Event Horizon

Event Horizon is a ledger-generic low-latency wake-up service, funded through Jupiter Faucet in ICP. The canonical ICP instance reads the ICP Ledger's legacy `query_blocks` log; non-ICP instances read compatible ICRC-1/ICRC-3 ledgers. Its target-aware poke reports each matching numeric subaccount or neuron nonce and the largest qualifying raw transfer amount. Subscribers keep authoritative ledger cursors and independent reconciliation; an Index is optional.

Canonical instances are ICP (alias `X`, live) and IO (intended alias `I`, planned). No IO backend or Ledger principal is claimed yet. Every compatible instance uses the same backend Wasm; immutable installation settings are `observed_ledger` and optional `sns_root`. Trigger thresholds use that token's decimals, while prices and cycles funding always use ICP.

The five declaration classes use these Jupiter Faucet memo forms. Canonical ICP
uses alias `X` only as an example; every instance uses its reviewed alias:

```text
<alias>.<subscriber>
<alias>.<subscriber>.<number>[:<amount>]
<alias>.<subscriber>.<start>-<end>[:<amount>]
<alias>.<subscriber>.n<nonce>[:<amount>]
<alias>.<subscriber>.n<start>-<end>[:<amount>]
```

The planned IO alias `I` is intended, not published. The full memo—including
alias, dot, subscriber, scope or range, and optional threshold—must fit
Jupiter's 32-byte memo limit; aliases are not assumed to be one character.
Numeric targets are canonical decimal `u64`. Ranges may begin anywhere but contain at most 256 inclusive targets and expand at admission into the existing watched-account map. `n<nonce>` selects a subscriber-controlled NNS or verified-SNS neuron staking account. Overlaps retain the lowest permanent threshold. `poke([])` signals global-only activity; a non-empty vector of target/max-amount matches takes precedence and is capped at 256 targets.

A subscriber may use several Event Horizon instances for several ledgers. Once authenticated, the Event Horizon caller identifies the ledger context of each poke; see the [subscriber guide](docs/subscriber-guide.md). Exact protocol semantics are normative in [SPEC.md](SPEC.md).

## Dynamic admission pricing

Event Horizon observes the CMC ICP/XDR conversion rate roughly daily and retains at most 1,461 UTC-day observations. With retained floor `F` and latest rate `C`:

```text
account price = ceil(10 × F / C) ICP
range price   = ceil(20 × F / C) ICP
global price  = ceil(100 × F / C) ICP
```

Prices freeze seven days before the next first-of-month 00:00 UTC boundary. A latest rate older than seven days at freeze carries the current prices forward. The daily observation is skipped rather than spending into the protected cycles reserve. Admission uses the current price when Event Horizon evaluates the exact Historian route total. Production exposes exactly the read-only `get_instance` and `get_pricing` queries; the certified frontend verifies the first against its reviewed static registry.

See [deploying an instance](docs/operations/deploying-an-instance.md), [acceptance observation](docs/operations/acceptance-observation.md), and the [subscriber guide](docs/subscriber-guide.md).

## Adaptive surplus funding

The production surplus destination is currently compiled as `None`, so funding behavior remains CMC-only. Once a fixed destination is selected and reviewed before controller removal, Event Horizon starts at 0% diversion. Each seven-day period whose hourly observed liquid-cycles minimum stays at least 150 T raises the fraction of future fee-net raw ICP eligible for diversion by 5 percentage points, up to 95%. A 100–150 T period lowers it by 5 points; a period below 100 T resets it to zero. No surplus leaves while the current balance is below 150 T.

Every raw-ICP balance is classified once. Event Horizon converts its retained share to cycles and confirms the CMC mint before the associated surplus share can be transferred to the immutable receiver's default ICP account. Direct donations enter this same common flow, and subscriber priority never depends on who supplied ICP. There is no treasury, withdrawal, destination, or policy API.

## Adaptive polling cadence

`T = 10^12 cycles`. Cadence decisions use **liquid cycles**—the balance immediately available to spend—not the total balance including cycles reserved for outstanding calls.

| Mode | Added delay after completed poll | Enter at | Exit below |
| ------------------ | -------------------------------: | -------: | ---------: |
| Reserve Protection | ordinary polling suspended | — | 1 T |
| Economy | 1 hour | 2 T | 1 T |
| Standard | 10 minutes | 5 T | 3 T |
| Fast | 2 minutes | 10 T | 6 T |
| Very Fast | 10 seconds | 25 T | 15 T |
| Continuous | 0 | 100 T | 60 T |

Event Horizon increases cadence immediately when a higher entry threshold is reached. When cycles fall, it remains in the current mode until that mode's lower exit threshold is crossed. For example, a canister at 5.27 T starting from a lower mode enters Standard mode. It remains Standard while its liquid balance is at least 3 T; below 3 T it falls back to an appropriate lower mode.

The values are added delays after completed polls, not exact wall-clock poll intervals: Ledger and inter-canister work also takes time. Continuous means Event Horizon deliberately inserts no delay after a completed poll. Polls still never overlap, and asynchronous IC execution naturally yields between calls.

Below 1 T liquid cycles, ordinary Ledger polling is suspended to protect protocol liveness. Funding and other recovery-oriented maintenance remain available. A canister already in Reserve Protection does not resume Economy until it reaches the 2 T Economy entry threshold.

Polling thresholds and surplus thresholds are separate mechanisms. Continuous polling begins at 100 T; the 150 T surplus-health threshold is used only by the currently disabled adaptive surplus policy. See [Operational backend](docs/operations/operational-backend.md) for timer and recovery details.

## Development

```bash
cargo run -p xtask -- test_unit
cargo run -p xtask -- test_all
npm run build:frontend
npm run test:frontend-unit
```

The xtask entry points refresh locked frontend dependencies with `npm ci` when needed. `test_unit` runs repository/static validation, source-manifest verification, ordinary Rust workspace tests (including xtask's runner tests), and frontend Node tests. `test_all` adds every intentionally ignored PocketIC scenario and prints one consolidated result.

## Source quality

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

## Release verification

```bash
./tools/scripts/security-scan
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
```

The final command leaves canonical deployable Wasms in `release-artifacts/`, verifies their manifest, and prints their exact uncompressed hashes for direct comparison with installed module hashes. The optional host-toolchain build remains `./tools/scripts/build-release`.

See the [`xtask` testing guide](tools/xtask/README.md), [testing documentation](docs/development/testing.md), and [reproducible-build documentation](docs/operations/reproducible-builds.md).

`Dockerfile.repro` is the canonical production build environment. See [`SPEC.md`](SPEC.md) for the normative protocol and [`CHECKPOINT.md`](CHECKPOINT.md) for provenance stages.

## Documentation

- [Documentation index](docs/README.md)
- [Architecture overview](docs/architecture/overview.md)
- [Development and testing](docs/development/testing.md)
- [Security](docs/security/dependency-scanning.md)
- [Operations and deployment](docs/operations/deployment.md)
- [Reproducible builds](docs/operations/reproducible-builds.md)
- [Backend canister documentation](canisters/event-horizon/README.md)
- [Frontend canister documentation](canisters/frontend/README.md)
- [Subscriber guide](docs/subscriber-guide.md)
