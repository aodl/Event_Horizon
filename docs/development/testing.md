# Testing

Event Horizon uses the same test vocabulary as Jupiter Faucet where the projects share concerns, without adding a local-replica layer or component matrix that this repository does not need.

## Commands

| Goal | Command |
| --- | --- |
| Prepare locked frontend dependencies | `cargo run -p xtask -- frontend_setup` |
| Repository and unit tests | `cargo run -p xtask -- test_unit` |
| Ignored PocketIC integration suite | `cargo run -p xtask -- test_pocketic_integration` |
| Complete behavioural suite | `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all` |

The test commands automatically run `npm ci` when `node_modules` is absent or stale relative to `package.json` and `package-lock.json`. No preliminary manual npm command is required.

`test_unit` runs repository/static protocol checks, source-manifest verification, ordinary locked Rust workspace tests (including xtask's parser/runner tests), and frontend Node unit tests. It excludes intentionally ignored PocketIC scenarios.

`test_pocketic_integration` first discovers the ignored library tests with:

```bash
cargo test --locked -p event-horizon-pocketic --lib -- --list --ignored
```

It then runs every discovered name serially in a separate Cargo test process,
using `--exact --ignored --nocapture --test-threads=1`. This gives every
scenario a fresh process and PocketIC lifecycle, reports each scenario
precisely, and avoids accumulating shared-server state across one long
integration process. The managed command sets `POCKET_IC_MUTE_SERVER=1` and
`RUST_TEST_THREADS=1`; discovery returning no ignored tests is a failure.

PocketIC is supplied by the Rust dependency and test harness; Event Horizon has no separately managed local-replica integration layer.

The runner streams child output, records suite durations and parsed Cargo/libtest or Node counts, continues to later independent suites after safe failures, and prints all failure details and rerun commands again in a final `Failures:` section. A successful zero-test selection is rejected.

To investigate one PocketIC scenario directly:

```bash
cargo test --locked -p event-horizon-pocketic --lib '<exact-test-name>' \
  -- --exact --ignored --nocapture --test-threads=1
```

One-way subscriber callbacks are tested with a bounded condition-based PocketIC wait. Exhaustion reports the expected condition, deterministic tick bound, elapsed time, the last mock-subscriber observation, and Event Horizon debug state.

## What PocketIC covers

The integration suite builds purpose-specific mock Wasms and exercises:

- prospective first-install bootstrap;
- exact 10 ICP account and 20 ICP range Historian admission;
- maximum 256-target range expansion, including ranges beginning above 255, resource reporting, matching, and upgrade health;
- range thresholds, overlap order, unfiltered overlap, sorted/deduplicated coalescing, and global precedence;
- incomplete or faulted Historian rejection and later natural admission on another Faucet payout;
- non-Faucet admission rejection;
- unfiltered and thresholded watched accounts;
- one poke carrying deterministic target/maximum-amount matches per subscriber per poll, bounded to 256 specific targets;
- NNS and verified-SNS neuron ownership, nonce derivation, ranges, and exact raw amounts;
- SNS Root transport and relationship failures that remain fail-closed and log once per uninterrupted failure episode;
- ordinary full-width `u64` targets, arbitrary-precision `Nat` callbacks, and the outbound 256-target bound;
- threshold loosening with per-target `max_amount` coalescing for subscriber-local policy;
- subscriber trap isolation and continued reader progress;
- explicit archive-gap skip-and-continue behavior and unexplained-hole retry semantics;
- generic ICRC-3 observation alongside canonical ICP legacy-log admission and observation;
- legacy ICP CMC top-up transfer;
- accepted transfer with lost response and duplicate-safe recovery;
- CMC `Processing` persistence across upgrade and later completion;
- explicit CMC refund and terminal-error autonomous clearing;
- retained-first ordering and CMC processing/refund/terminal cancellation of planned surplus;
- frozen surplus destination and memo across retained-leg progress, upgrades, ambiguous accepted-transfer recovery, destination replacement, and destination disablement;
- second-gate cancellation when liquid cycles fall after planning;
- new ICP exclusion from an in-flight plan and later classification;
- surplus success, lost-response duplicate recovery, clean rejection, insufficient funds, bad fee, future timestamp, `TxTooOld`, and pending-transfer upgrade;
- subscription admission after the original Faucet ICP has flowed through funding or surplus transfers;
- destination-disabled equivalence and zero policy;
- fresh-install `FundingState::Idle`, CMC-only `None`, split-plan `Some(PlannedSurplus)`, and current-Wasm upgrades of every pending funding phase;
- subscription, cursor, verified SNS profile, and scheduler persistence across upgrade;
- timer-wrapper recovery after real post-await continuation traps in polling, pricing, and funding, including preserved pending funding identity and non-overlapping ownership;
- Reserve Protection checks at scheduled poll entry, including Economy hysteresis and resumption above 2 T;
- the certified frontend response, transition asset URLs, revalidation headers, and absence of debug application methods from production Wasm;
- deterministic stale-frontend-response rejection under reordered success, failure, planned-entry, same-profile/different-backend, and verification-failure cases;
- a bounded capacity matrix: global and specific fan-out at 256 and 1,024 records, a 4,096-block/16-page backlog through the scheduler, the existing 256-target callback bound, values above `u64::MAX`, and a 2,048-decimal-digit `Nat` fixture.

The capacity matrix uses batched debug-only fixture setup and the production polling path. Synthetic principals measure dispatch pressure as attempted/accepted one-way calls; they are not described as confirmed subscriber execution. The large-`Nat` case uses the real mock subscriber and confirms execution. PocketIC exposes cycle and memory deltas used in the validation report, but not a sound single aggregate instruction count across the poll's many asynchronous messages. The tested matrix is a finite envelope, not evidence of unlimited capacity.

## Source quality

Formatting and Clippy are explicit gates rather than xtask commands:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

## Release gates

Behavioural testing is separate from dependency security and artifact production:

```bash
./tools/scripts/security-scan
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
```

See [dependency scanning](../security/dependency-scanning.md), [reproducible builds](../operations/reproducible-builds.md), and the [xtask guide](../../tools/xtask/README.md).
