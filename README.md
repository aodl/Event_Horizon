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

## Validation

```bash
npm ci
cargo run -p xtask -- check
cargo run -p xtask -- pocketic
cargo run -p xtask -- local-smoke
cargo run -p xtask -- security
cargo run -p xtask -- release
cargo run -p xtask -- repro
icp build -e local
```

`Dockerfile.repro` is the canonical production build environment. See [`SPEC.md`](SPEC.md) for the normative protocol and [`CHECKPOINT.md`](CHECKPOINT.md) for provenance stages.

## Documentation

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/subscriber-guide.md`](docs/subscriber-guide.md)
- [`docs/economics-pricing.md`](docs/economics-pricing.md)
- [`docs/trust-model.md`](docs/trust-model.md)
- [`docs/frontend.md`](docs/frontend.md)
- [`docs/deployment.md`](docs/deployment.md)
- [`docs/controller-removal.md`](docs/controller-removal.md)
