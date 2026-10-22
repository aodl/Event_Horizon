# Event Horizon

![img](/canisters/frontend/public/og/event-horizon-preview-v1.jpg)

Event Horizon is a ledger-generic low-latency wake-up service funded by
productive endowments through Jupiter Faucet. Endowed ICP principal contributes
to Jupiter's productive neuron stake; it does not become Event Horizon's liquid
operating balance. Recurring maturity-derived Faucet payouts are sent to Event
Horizon as raw ICP, pooled, and converted into cycles. The canonical ICP
instance reads the ICP Ledger's legacy `query_blocks` log; non-ICP instances
read compatible ICRC-1/ICRC-3 ledgers. Its payload-bearing poke reports each
matching numeric subaccount or neuron nonce with its first qualifying Ledger block index and largest qualifying raw
transfer amount. Scoped subscribers should normally use target-aware `poke`
to filter locally approved targets and amount thresholds before expensive work:
any patron can fund a subscription without the receiving canister's consent.
A trusted caller is not proof of wanted activity. Zero-argument `poke()` remains
Candid-compatible for intentional general wake-ups with cheap, bounded/coalesced
work; it discards those filtering hints. See the [subscriber guide](docs/subscriber-guide.md).
Subscribers keep authoritative ledger cursors and independent reconciliation;
an Index is optional.

The canonical ICP backend is live; this release prepares `E` routing, whose Jupiter deployment operators must verify before offering alias-dependent funding. IO (intended alias `I`) remains planned. No IO backend or Ledger principal is claimed yet. Every compatible instance uses the same backend Wasm; immutable installation settings are `observed_ledger`, optional `sns_root`, and optional `surplus_canister`. Trigger thresholds use the observed token's decimals, while prices, cycles funding, and any surplus remain ICP.

The five declaration classes use these Jupiter Faucet memo forms. Canonical ICP
uses alias `E` only as an example; every instance uses its reviewed alias:

```text
<alias>.<subscriber>
<alias>.<subscriber>[@<subject>].<number>[:<amount>]
<alias>.<subscriber>[@<subject>].<start>-<end>[:<amount>]
<alias>.<subscriber>[@<subject>].n<nonce>[:<amount>]
<alias>.<subscriber>[@<subject>].n<start>-<end>[:<amount>]
```

The planned IO alias `I` is intended, not published. The full memo—including
alias, dot, subscriber, scope or range, and optional threshold—must fit
Jupiter's 32-byte memo limit; aliases are not assumed to be one character.
Numeric targets are canonical decimal `u64`. Ranges may begin anywhere but contain at most 256 inclusive targets and expand once at admission into indexed watched-account relations. `n<nonce>` selects a subject-controlled NNS or verified-SNS neuron staking account. Overlaps retain the lowest permanent threshold. Self/global `poke` remains compatible with older receivers; external-only activity uses `poke_subjects`. A non-empty vector of target/max-amount matches takes precedence and shares a cap of 256 subject-qualified hints.

A subscriber may use several Event Horizon instances for several ledgers. Once authenticated, the Event Horizon caller identifies the ledger context of each poke; see the [subscriber guide](docs/subscriber-guide.md). Exact protocol semantics are normative in [SPEC.md](SPEC.md).

## Mainnet canisters

| Component / CLI name | Canister ID | Component documentation |
| --- | --- | --- |
| Backend: `event_horizon` | `eo6ei-gaaaa-aaaar-qchra-cai` | [Backend README](canisters/event-horizon/README.md) |
| Frontend: `event_horizon_frontend` | `ej7c4-lyaaa-aaaar-qchrq-cai` | [Frontend README](canisters/frontend/README.md) |

The public site is [www.event-horizon-icp.com](https://www.event-horizon-icp.com/);
the frontend also has its [canister URL](https://ej7c4-lyaaa-aaaar-qchrq-cai.icp.net/).
These IDs identify the existing canonical ICP deployment, not local replicas or
future additional instances. `icp.yaml` declares the CLI names;
`.icp/data/mappings/ic.ids.json` resolves them in the `ic` environment. This
operator-local file is ignored by Git, so restore and verify it on a fresh
checkout using the [deployment runbook](docs/operations/deployment.md#canister-name-resolution).

## Dynamic admission pricing

Event Horizon observes the CMC ICP/XDR conversion rate roughly daily and retains at most 1,461 UTC-day observations. With retained floor `F` and latest rate `C`:

```text
account price = ceil(10 × F / C) ICP
range price   = ceil(20 × F / C) ICP
global price  = ceil(1000 × F / C) ICP
```

Prices freeze seven days before the next first-of-month 00:00 UTC boundary. A latest rate older than seven days at freeze carries the current prices forward. The daily observation is skipped rather than spending into the protected cycles reserve. Admission uses the current price when Event Horizon evaluates the exact Historian route total. Production exposes the read-only `get_instance`, `get_pricing`, and local declaration `preview_subscription` queries; the certified frontend verifies the first against its reviewed static registry.

The initial reference requirements are therefore 10 ICP for an account, 20 ICP
for a range, and 1,000 ICP for a global subscription. These are reference bases,
not permanently fixed displayed prices: each class is independently rounded by
the formula above. A qualifying declaration becomes permanent only after Event
Horizon observes a Faucet payout and Historian verifies the complete cumulative
endowment for the exact route. Additional endowment does not buy priority.

The launch plan separately contemplates a 3,000 ICP productive protocol
endowment for baseline recurring funding and a liquid-cycles bootstrap while
initial payouts are pending. The planned endowment is neither confirmed
deposited nor immediately spendable, and service sustainability depends on
actual payouts, conversion rates, workload, fees, and cycle costs. See
[Economics and pricing](docs/architecture/economics-pricing.md) for the funding
flow and limitations.

See [deploying an instance](docs/operations/deploying-an-instance.md), [acceptance observation](docs/operations/acceptance-observation.md), and the [subscriber guide](docs/subscriber-guide.md).

## Adaptive surplus funding

`surplus_canister = null` disables diversion and keeps funding CMC-only. `opt principal` enables the existing adaptive policy for that instance and derives the recipient's default all-zero-subaccount ICP AccountIdentifier; it does not immediately divert value. The canonical ICP install arguments currently use `null`. Each seven-day period whose hourly observed liquid-cycles minimum stays at least 150 T raises the fraction of future fee-net raw ICP eligible for diversion by 5 percentage points, up to 95%. A 100–150 T period lowers it by 5 points; a period below 100 T resets it to zero. No surplus leaves while the current balance is below 150 T.

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

Polling thresholds and surplus thresholds are separate mechanisms. Continuous polling begins at 100 T; the 150 T surplus-health threshold is used only when an instance has configured a surplus recipient. See [Operational backend](docs/operations/operational-backend.md) for timer and recovery details.

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

Cycles-based cadence thresholds can be measured and modelled locally through
the opt-in [cadence calibration framework](docs/development/cadence-calibration.md).
It uses PocketIC and synthetic cycles only; it is not part of ordinary tests and
does not change the production threshold table.

`Dockerfile.repro` is the canonical production build environment. See [`SPEC.md`](SPEC.md) for the normative protocol and [`CHECKPOINT.md`](CHECKPOINT.md) for provenance stages.

## Documentation

- [Documentation index](docs/README.md)
- [Architecture overview](docs/architecture/overview.md)
- [Funding, economics, and pricing](docs/architecture/economics-pricing.md)
- [Development and testing](docs/development/testing.md)
- [Security](docs/security/dependency-scanning.md)
- [Operations and deployment](docs/operations/deployment.md)
- [Reproducible builds](docs/operations/reproducible-builds.md)
- [Backend canister documentation](canisters/event-horizon/README.md)
- [Frontend canister documentation](canisters/frontend/README.md)
- [Subscriber guide](docs/subscriber-guide.md)
