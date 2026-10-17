# Testing

Event Horizon uses the same test vocabulary as Jupiter Faucet where the projects share concerns, without adding a local-replica layer or component matrix that this repository does not need.

## Commands

| Goal | Command |
| --- | --- |
| Prepare locked frontend dependencies | `cargo run -p xtask -- frontend_setup` |
| Repository and unit tests | `cargo run -p xtask -- test_unit` |
| Ignored PocketIC integration suite | `cargo run -p xtask -- test_pocketic_integration` |
| Complete behavioural suite | `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all` |
| Opt-in cadence calibration | `./tools/scripts/calibrate-cadence smoke` |

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

The resource-intensive cadence experiments are a separate binary, not ignored
tests, so `test_all` does not discover or execute them. See
[cadence calibration](cadence-calibration.md) for bounded smoke/full
measurements, offline modelling, report generation, and the tracked example
input.

The runner streams child output, records suite durations and parsed Cargo/libtest or Node counts, continues to later independent suites after safe failures, and prints all failure details and rerun commands again in a final `Failures:` section. A successful zero-test selection is rejected.

To investigate one PocketIC scenario directly:

```bash
cargo test --locked -p event-horizon-pocketic --lib '<exact-test-name>' \
  -- --exact --ignored --nocapture --test-threads=1
```

One-way subscriber callbacks are tested with a bounded condition-based PocketIC wait. Exhaustion reports the expected condition, subscriber principal and receiver style, deterministic tick bound, elapsed time, the last mock-subscriber observation, and Event Horizon debug state. Positive delivery assertions require a fresh receiver-side counter increment and verify the recorded caller; Event Horizon's accepted one-way-call counter alone is not treated as execution evidence.

Ordinary Rust tests distinguish a zero-argument Candid sequence, one `null`
argument, and one empty match vector, then cover both zero-argument and
vector-taking decoders with empty, nonempty, and 256-match payloads. A direct
PocketIC case sends identical encoded vectors through the two fixtures' real
CDK-generated entry points. Production-path cases then use ordinary
Faucet/Historian admission and Ledger polling with both receiver styles in one
instance, across the shared ICP reader and a separate ICRC-3 observed ledger.

## What PocketIC covers

The integration suite builds purpose-specific mock Wasms and exercises:

- prospective first-install bootstrap;
- exact 10 ICP account, 20 ICP range, and 1,000 ICP global Historian
  admission, including the one-e8s-below global boundary and cumulative
  600+399/600+400 ICP route totals;
- maximum 256-target range expansion, including ranges beginning above 255, resource reporting, matching, and upgrade health;
- range thresholds, overlap order, unfiltered overlap, sorted/deduplicated coalescing, and global precedence;
- incomplete or faulted Historian rejection and later natural admission on another Faucet payout;
- non-Faucet admission rejection;
- unfiltered and thresholded watched accounts;
- one poke carrying deterministic target/maximum-amount matches per subscriber per poll, bounded to 256 specific targets;
- zero-argument wake-up receivers alongside payload-aware receivers, including
  real entry-point arity rejection, global and specific delivery, caller
  authentication evidence, coalescing, precedence, and no-new-work behavior;
- NNS and verified-SNS neuron ownership, nonce derivation, ranges, and exact raw amounts;
- SNS Root transport and relationship failures that remain fail-closed and log once per uninterrupted failure episode;
- ordinary full-width `u64` targets, arbitrary-precision `Nat` callbacks, and the outbound 256-target bound;
- threshold loosening with per-target `max_amount` coalescing for subscriber-local policy;
- subscriber trap isolation and continued reader progress;
- table-driven archive interval validation (partial, adjacent, overlapping,
  repeated, gapped, zero-length, over-broad, and overflowing ranges), awkward
  empty legacy first indices, no-progress and genuine-hole rejection;
- shared ICP, separate ICP admission, and generic ICRC-3 multi-page
  archive-only catch-up through a pinned boundary, including one real Economy
  scheduler execution and exact live-tail callback payloads;
- structural response rejection before archive mutation, durable proved
  archive prefixes before malformed live data, page-level admission commits,
  and identical/conflicting ICRC-3 duplicate IDs;
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
- subscription, cursor, verified SNS profile, pricing, and scheduler persistence
  across ordinary current-schema upgrade, including archive backlog recovery;
- the tracked pre-policy Wasm fixture establishing old 10/20/100 pricing state,
  including a diagnostic ordinary upgrade that preserves reader state but
  exposes the unsafe old-global/new-divisor interpretation, followed by a local
  controlled reinstall that proves the state is discarded, fresh
  10/20/1,000 pricing is established, and the new prospective reader does not
  replay historical admission activity;
- timer-wrapper recovery after real post-await continuation traps in polling, pricing, and funding, including preserved pending funding identity and non-overlapping ownership;
- Reserve Protection checks at scheduled poll entry, including Economy hysteresis and resumption above 2 T;
- the certified frontend response, transition asset URLs, revalidation headers, and absence of debug application methods from production Wasm;
- deterministic stale-frontend-response rejection under reordered success,
  failure, planned-entry, same-profile/different-backend, and
  verification-failure cases, plus synchronous removal of the previous memo
  and funding instruction while a new live instance is pending;
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

## Landing ledger counter

```bash
npm run build:frontend
npm run test:frontend-unit
node tools/scripts/validate-ledger-counter.mjs
# Explicitly opt in to anonymous, read-only mainnet queries:
node tools/scripts/check-ledger-count.mjs --live
node tools/scripts/validate-ledger-counter.mjs --live
```

The focused Node suites inject clocks, timers, frames, queries and transport.
They run offline without real sleeps. They cover independent full Candid wire
schemas using the pinned SDK decoder, request scalar/arity checks, exact BigInt
interpolation and deterministic property cases, single-flight deadlines,
initialization eviction, real fetch abort ownership, late completions, freshness,
contradictory samples, selection races, pricing isolation and artwork lifecycle.
The shared page fixtures preserve the existing memo/pricing race assertions.

The browser harness uses development-only Playwright 1.61.1. Install its browser
with `npx playwright install chromium` when needed, or set
`COUNTER_BROWSER_EXECUTABLE` to an available Chromium executable. OS libraries
must already be available; the script does not install system packages or use
privilege escalation. Set `TMPDIR` to a suitable writable temporary directory.
The harness starts a loopback server with production CSP/cache headers and serves
the actual built bundle for offline failure/planned/artwork checks. A separately
compiled **test-only** entry invokes production DOM integration with injected
queries and clocks for deterministic state/layout checks. It has no place in
`public/`, the production bundle, or certified assets, and never bypasses SDK
verification. Synthetic generic samples are protocol integration evidence, not a
claim that IO or another generic instance is live. `--live` additionally opens
the unmodified built page with real production-verification ledger queries.

Screenshots, request/console evidence, accessibility snapshot and browser result
are written to ignored `observations/ledger-counter/browser/`. The optional
Node live smoke makes exactly two logical attempts, with ten-second deadlines
and a two-second gap, using the existing registry and pinned agent. Its JSON
records UTC receipt time, principal, method, request shape and successful totals
or failures. No transactions, wallet connection or backend operation are used.

The existing PocketIC certified-frontend test asserts the new hero markup and
compares query-suffixed served bundle/CSS bytes with the source assets while
retaining certificate, CSP and revalidation assertions. Run the complete
`test_all` gate after rebuilding; pure JavaScript tests alone do not prove what
the frontend Wasm embeds. See the [validation report](../codex/ledger-counter-validation.md)
for browser/device limits and retained evidence.

## Hero branding and recent-rate regressions

Run `npm run test:frontend-unit` for dedicated `compact-heading.test.mjs` and
`ledger-rate.test.mjs` plus existing counter/client/integration and builder tests.
`node tools/scripts/validate-ledger-counter.mjs` exercises the built page and a
separate injected-query page with the same production controllers, with no
production test switches. Use the installed pinned Playwright Chromium (or set
`COUNTER_BROWSER_EXECUTABLE` for its installed path).

The browser matrix covers 1440×900, 1280×800, 1024×768, 1000×800 branding, 1000×900 and 1000×1000 near-square media regressions, the reported 873×804 → 873×940 height-only transition, 844×390 landscape, 390×844, 360×640, 320×568 and 200% text. Assertions
check observed heading/compact visibility, dynamic scroll/resize, native anchor,
bounded bottom-edge title peek (96px retained, 129px hidden), art-first compact text/facts, independently positioned ledger statistic, absence of the removed manual pause control, unboxed styling, fourth-fact alignment,
overflow, rate pending/nonzero/zero/tiny values, delay/hidden suspension, long generic totals,
reduced motion, SVG timeline/CSS pause, quiet accessibility and cleanup. Evidence
is ignored under `observations/hero-facts/` (earlier evidence remains under `observations/ledger-counter-calm/`, `observations/compact-title-aspect-gate/` and `observations/ledger-counter-refinement/`). Browser emulation is
not physical-device or cross-engine performance proof. Full `test_all` still
runs every discovered ignored PocketIC scenario, including exact certified asset
bytes and query-suffixed routing. See the
[hero refinement validation report](../codex/ledger-hero-refinement-validation.md) and the
[compact hero restoration validation](../codex/compact-hero-restoration-validation.md).

The counter's calmer refresh regressions allow a successful response through
30 seconds, keep transient retries quiet, and show a persistent delay only at
three minutes without acceptance. The browser checks exact count, label and
heading rectangles through a slow response, isolated failure, delayed notice
and recovery at 1440×900, 1024×768, 873×804, 390×844, 360×640 and 320×568. The
permanent notice slot must keep these rectangles unchanged. Transport abort,
single flight, backoff, rate invalidation and contradictory-data checks remain
covered by the existing deterministic suites.

## Hero fact navigation

`hero-facts.test.mjs` checks the four native fragment links, exact labels and
local icons, unique labelled destinations, section order, separate external
actions, original asset hashes and inactive SVG contents. The existing ignored
PocketIC certified-frontend scenario verifies exact icon bytes and SVG/ICO media
types alongside the existing PNG/favicon checks.

`node tools/scripts/validate-ledger-counter.mjs` exercises the built page and
the separate query fixture. It checks locally loaded decorative icons, accessible
link/region names, keyboard order/focus outlines, native fragment/history/direct
loading, smooth versus reduced-motion scrolling, editorial targets, 200% text,
and the existing hero title/art/counter regressions. Representative sizes include
1440×900, 1280×800, 1024×768, 873×804, 844×390, 390×844, 360×640 and 320×568.
Evidence is ignored under `observations/hero-facts/browser/`. See
[hero-facts validation](../codex/hero-facts-validation.md).

## Instance operating status

`npm run test:frontend-unit` includes `operating-tier.test.mjs`,
`operating-status-client.test.mjs` and `operating-status.test.mjs`. They cover
production threshold parity, explicit hysteretic modes, exact large cycle
arithmetic, Reserve/highest-tier progress, native query Candid projection,
reviewed effective routing, agent reuse, transport cancellation during
initialization/query, late responses, timeout, planned selection, visibility,
configuration mismatch and failure isolation from pricing/the memo builder.

`node tools/scripts/validate-ledger-counter.mjs` also checks the operating panel
at desktop, phone and narrow widths with separate injected query fixtures.
Its opt-in `--live` branch reads public native status/logs using the actual
built bundle and unchanged verification configuration. This is read-only and
must remain separate from deterministic fixture evidence. The certified-asset
PocketIC regression checks the operating markup as well as exact served
bundle/CSS bytes. See [operating-status validation](../codex/operating-status-validation.md)
for evidence and the limitations of dated mode/liquid readings.
