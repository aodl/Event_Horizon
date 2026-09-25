# Event Horizon

Event Horizon is a low-latency, best-effort ICP Ledger wake-up service funded by perpetual Jupiter Faucet endowments. It reads the live ICP Ledger directly and calls a subscriber's `poke(vec nat8)` endpoint. Subscribers keep authoritative Ledger or Index cursors and an independent reconciliation timer; a missed poke affects latency rather than correctness.

It supports five exact Jupiter Faucet memo forms:

```text
X.<subscriber>                         # global Ledger activity
X.<subscriber>.<subaccount>            # every incoming account transfer
X.<subscriber>.<subaccount>:<amount>   # inclusive account threshold
X.<subscriber>.<start>-<end>           # every transfer in an inclusive range
X.<subscriber>.<start>-<end>:<amount>  # inclusive threshold throughout a range
```

Ranges satisfy `0 <= start < end <= 255` and expand at admission into the existing watched-account map. Overlaps retain the lowest permanent threshold. `poke([])` signals global activity without a more-specific match; one sorted unique non-empty vector of actual matched subaccounts takes precedence.

## Dynamic admission pricing

Event Horizon observes the CMC ICP/XDR conversion rate roughly daily and retains at most 1,461 UTC-day observations. With retained floor `F` and latest rate `C`:

```text
account price = ceil(10 × F / C) ICP
range price   = ceil(20 × F / C) ICP
global price  = ceil(100 × F / C) ICP
```

Prices freeze seven days before the next first-of-month 00:00 UTC boundary. A latest rate older than seven days at freeze carries the current prices forward. The daily observation is skipped rather than spending into the protected cycles reserve. Admission uses the current price when Event Horizon evaluates the exact Historian route total. The backend's only production application method is the read-only `get_pricing` query; the certified frontend calls that query directly from its bundled browser client.

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

Polling thresholds and surplus thresholds are separate mechanisms. Continuous polling begins at 100 T; the 150 T surplus-health threshold is used only by the currently disabled adaptive surplus policy. See [Operational backend](docs/operational-backend.md) for timer and recovery details.

## Development and validation

```bash
npm ci
cargo run -p xtask -- check
cargo run -p xtask -- test-all
cargo run -p xtask -- validate
```

`check` is the normal source-quality gate. `test-all` adds every intentionally ignored PocketIC scenario. The intentionally expensive `validate` command runs source checks, all PocketIC scenarios, dependency-security checks, two clean reproducibility builds, and finally produces the canonical Docker-built Wasms under `release-artifacts/`.

To rebuild only the exact production artifacts:

```bash
cargo run -p xtask -- canonical
```

This verifies the artifact manifest and prints the exact uncompressed Wasm hashes for direct comparison with mainnet module hashes. See the [`xtask` command guide](tools/xtask/README.md) for the complete command matrix and [reproducible-build documentation](docs/reproducible-builds.md) for verification details.

`Dockerfile.repro` is the canonical production build environment. See [`SPEC.md`](SPEC.md) for the normative protocol and [`CHECKPOINT.md`](CHECKPOINT.md) for provenance stages.

## Documentation

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/subscriber-guide.md`](docs/subscriber-guide.md)
- [`docs/economics-pricing.md`](docs/economics-pricing.md)
- [`docs/trust-model.md`](docs/trust-model.md)
- [`docs/frontend.md`](docs/frontend.md)
- [`docs/deployment.md`](docs/deployment.md)
- [`docs/controller-removal.md`](docs/controller-removal.md)
