# Cadence calibration

This opt-in framework measures the existing Event Horizon polling implementation in PocketIC, extrapolates those measurements through a deterministic offline model, and generates a threshold comparison. The PocketIC measurement runner is opt-in; offline unit/regression tests run with normal workspace tests. It does not contact a deployed canister or change the production cadence table.

The implementation follows the pinned PocketIC 15.0.0 API. PocketIC provides deterministic canister execution, explicit simulated-time advancement, one-round `tick()` execution, and actual fixture canister cycle balances; see the [ICP PocketIC guide](https://docs.internetcomputer.org/guides/testing/pocket-ic/) and [PocketIC 15.0.0 API](https://docs.rs/pocket-ic/15.0.0/pocket_ic/struct.PocketIc.html). Canister time is fixed within a message, as described in the [ICP timer documentation](https://docs.internetcomputer.org/guides/backends/timers/). These properties make PocketIC suitable for local comparisons, not proof of mainnet latency or subnet cost.

## Layers

The three layers remain explicit in every output:

1. `smoke` and `full` execute the real debug/test Event Horizon Wasm and existing mock canisters in PocketIC. They write measured cycle deltas, completed calls, cursors, callback counters, simulated timestamps, caps, and provenance.
2. `model` derives architecture-specific polling coefficients and qualified maintenance residuals from a selected `measurements.json`, then performs integer/fixed-point 30-, 90-, and 365-day projections. Long horizons do not execute millions of PocketIC messages.
3. `report` reads both datasets and renders the measured results, extrapolations, assumptions, policy objectives, candidate comparison, recommendation, and limitations.

The tracked [model configuration](../../tests/cadence-calibration/fixtures/model-config.json) separately identifies assumed yield/conversion inputs, workload definitions, candidate threshold parameters, Continuous effective-poll-interval sensitivity, equal starting balances, and runway objectives. Each workload states its Ledger architecture, global subscriber count, specific-match distribution, and productive-endowment backing. The tracked [example measurements](../../tests/cadence-calibration/fixtures/example-measurements.json) allow the offline layers to run without first running PocketIC; because that compact fixture has no generic-reader or integrated-scheduler rows, its output explicitly marks those profiles unavailable rather than inventing coefficients. Neither file is a mainnet observation.

## Commands

Run from the repository root:

```bash
./tools/scripts/calibrate-cadence smoke \
  --seed 20261011 \
  --output observations/cadence/smoke-20261011

./tools/scripts/calibrate-cadence full \
  --seed 20261011 \
  --output observations/cadence/full-20261011

./tools/scripts/calibrate-cadence model \
  --input observations/cadence/full-20261011/measurements.json \
  --config tests/cadence-calibration/fixtures/model-config.json \
  --output observations/cadence/full-20261011 \
  --seed 20261011

./tools/scripts/calibrate-cadence report \
  --input observations/cadence/full-20261011
```

For an offline-only review:

```bash
./tools/scripts/calibrate-cadence model \
  --input tests/cadence-calibration/fixtures/example-measurements.json \
  --config tests/cadence-calibration/fixtures/model-config.json \
  --output observations/cadence/example \
  --seed 20261011
./tools/scripts/calibrate-cadence report --input observations/cadence/example
```

Measurement bounds may be tightened with `--max-ticks`, `--max-polls`, `--max-blocks`, and `--max-wall-seconds`. A wait, scenario, or overall run that reaches a cap fails visibly; it is never reported as a complete result. In particular, Continuous execution is bounded by completed polls and ticks.

The script sets `DFX_IDENTITY=codex_local` defensively, although it uses PocketIC rather than `dfx` or `icp-cli`. It performs no network deployment, top-up, transfer, reinstall, upgrade, or controller operation.

## Measurement profiles

Smoke checks the schema and execution path with Economy and Continuous idle polls, Reserve Protection recovery, canonical ICP and generic ICRC-3 light/multi-page traffic, 0/10 global fan-out, and exact hysteresis parity.

Full adds:

- all five active tier timers with repeated idle polls;
- three additional Very Fast idle trials from fresh equivalent PocketIC fixtures for repeatability;
- canonical ICP shared-scan and separate generic ICRC-3 readers at idle, 1, 5, and 20 blocks/second plus 1,024- and 4,096-block bursts;
- archive-prefix skipping and live-tail decoding for both reader architectures;
- settled global fan-out at 0, 1, 10, 100, 256, and 1,024 subscribers;
- matching and non-matching specific subscriptions, coalescing, mixed global/specific delivery, a representative expanded range, and unified `poke(PokeNotification)` execution;
- simulated transaction-to-receiver latency in every active tier plus a burst/coalescing case;
- every threshold immediately below, exactly at, and immediately above both entry and exit boundaries, followed by a top-up/decline/interruption/recovery sequence;
- actual PocketIC balances at 1, 2, 5, 10, 25, 60, 100, 200, and 1,000 T.

The framework reuses the existing debug scheduler controls: all three production timer wrappers are started and allowed to complete one warm-up before a timer-driven measurement window. The Standard, Fast, Very Fast, and Continuous baseline windows finish before the next funding or pricing timer. Economy's one-hour poll interval necessarily coincides with hourly funding maintenance; that row is marked maintenance-inclusive and excluded from the isolated empty-poll coefficient. A separate Fast scenario measures the initial all-lane scheduler work, and an integrated scenario below the actual 1 T floor proves that Reserve Protection suspends ordinary polling while raw-ICP funding work and recovery remain available. The balance override is used only for exact hysteresis observations, never for depletion measurements.

Installation, profile discovery, initial cursor establishment, and pricing setup occur before measurement windows. Per-poll samples are taken only after the asynchronous polling lane completes. Fan-out balance samples additionally wait for rejection/refund traffic to settle. Only the Event Horizon backend balance is subtracted; mock Ledger and subscriber execution costs are excluded.

## Outputs

Each ignored `observations/cadence/<run-id>/` directory contains:

- `measurements.json`: complete structured PocketIC evidence and provenance;
- `measurements.csv`: comparable scenario rows;
- `simulations.json`: assumptions, calibrated coefficients, time-in-tier, funding, burn, liquidity, backlog, latency, and transition projections;
- `threshold-comparison.csv`: candidate/workload/horizon comparisons;
- `report.md`: human-readable methodology, results, recommendation, and uncertainty.

`measurements.json` records the git revision and dirty state, debug/test Wasm hash, PocketIC version, topology/cost schedule, seed, fixture state, workload, completed polls/calls/pages, cursors, pokes, backend balances, maintenance inclusion, and completion/cap state. Generated observations are ignored and do not enter the canonical source manifest or release artifacts.

## Input integrity and queue interpretation

The model supports in-place generation and a separate output directory. Before copying either measurement file it resolves existing source and destination paths, skipping identical canonical files; Unix device/inode checks also recognize hard links. Relative/absolute paths, dot components, directory symlinks and existing file symlinks are covered. Skipping preserves bytes, modification time and permissions; separate copies use standard filesystem copy semantics. Unexpected resolution/copy failures include both paths and stop the command. Concurrent external replacement of files during a run is outside the tool's contract.

Record SHA-256 hashes of both inputs before and after modelling. For alias/failure experiments use a disposable copy, retaining the original full dataset. Repeated model/report runs with identical paths, configuration and seed are deterministic.

`maximum_active_pre_poll_pending_blocks` measures waiting immediately before completed active polls, including peaks inside skipped batches. `maximum_active_post_poll_outstanding_blocks` measures residual processing lag after those polls. Transition snapshots record outstanding blocks at the first downgrade and first reserve entry. `additional_reserve_backlog_blocks` counts new blocks arriving during all Reserve Protection episodes; later recovery may drain them. `ending_backlog_blocks` includes all remaining work, and `maximum_backlog_blocks` includes waiting and suspended-poll accumulation. The existing ending-latency heuristic is conditional on active polling; it cannot estimate recovery time while Reserve Protection suspends polling.

At one block/second and the assumed 4,096-block capacity, Economy's 3,600, Standard's 600 and Fast's 120 waiting blocks clear each completed poll. Slower response is not persistent throughput backlog. Arrivals exceeding capacity produce growing post-poll lag; suspended polling produces reserve backlog. Backlog does not prove archival loss: live retention and explicit archive evidence determine whether history becomes unavailable. The protocol remains best effort and never traverses archives.

## Financial interpretation

Each workload represents one independent Event Horizon instance and specifies its own protocol productive endowment and subscriber backing. Canonical workloads assume the planned 3,000 ICP protocol endowment; the generic ICRC-3 workload has no protocol seed and receives payouts only from its own subscriber endowments. Reference productive endowments are 10 ICP per account declaration, 20 ICP per range declaration, and 1,000 ICP per global declaration. The admission-price multiplier must be positive and at most 100% because the rolling minimum rate cannot exceed the current eligible rate. Each class is rounded independently up to whole ICP (at 62.5%: 7, 13, and 625 ICP). These are minimum qualifying amounts; a patron may voluntarily endow more. The default funding calculation assumes patrons endow exactly the qualifying minima; voluntary extra principal is a separate assumption and is not represented by a multiplier above 100%. The expected workload's one global and five specific subscribers have corresponding backing; the stress workload deliberately decouples subscriber counts from subscription funding. The model never credits principal as liquid ICP or cycles. Only maturity-derived payout assumptions are converted using a separate ICP/XDR input and the standard 1 T cycles per XDR relationship.

The default 7% gross and 5.6% effective payout APYs are scenarios, not promises. The model covers no receipts; daily, weekly, monthly, and irregular receipts; seven-, 30-, and 90-day interruptions; delayed lump-sum recovery; 2.8% low yield; and zero effective yield. Recognition delay, conversion delay/fee, delivery shortfall, optional surplus diversion, subscription counts, workloads, policies, and the assumed effective Continuous interval are editable in the JSON configuration. The configured conversion fee is deducted from each delivered payout, so accumulated or delayed receipts remain distinct from regular small receipts.

Simulation schema 3 replaces the former `sustainable` field with `window_funding_covers_expenditure`: actual receipts cover expenditure over the selected horizon, regardless of liquidity. Funding surplus and shortfall are reported separately. Liquidity means no exhaustion or Reserve Protection entry and an ending balance of at least 1 T; queue depth is not part of that financial classification. Minimum/ending cycles, time in each tier, reserve entry, exhaustion and suspended seconds describe reserve safety.

The separate `fixed_tier_economics` rows estimate hypothetical continuing workloads in each active tier, including Continuous completed-poll intervals of 1, 2, 5 and 10 seconds. They do not assume that an adaptive path remains in its initial tier. Income uses productive principal, effective payout APY, shortfall, diversion, ICP/XDR conversion and one conversion fee per weekly payout. Its mature annual rate is `365 / 7` times the integer fee-net weekly receipt. Initial recognition and conversion delay affect window receipts, not principal or the mature rate. Payouts represent completed accrual periods: the first regular receipt follows recognition plus conversion delay plus one payout interval. Maintenance is hourly funding and daily pricing; ordinary costs include empty polls, processed blocks, extra pages and coalesced global/specific dispatches. Annual generated/processed counts expose capacity-limited fixed-tier rows; affordability does not establish throughput adequacy. All annual cost calculations use checked integer arithmetic. The fixed diversion fraction is an effective retained-income assumption, and `conversion_fee_e8s` is the explicit per-delivery fee budget. The model does not reproduce the production surplus epochs, execution refunds or additional unconfigured fee legs.

A first-year deficit can coexist with a positive mature annual margin. Equal gross receipts on different schedules can produce different minima; more frequent conversion also incurs more fees. A positive annual margin alone does not establish adequate bootstrap liquidity or uninterrupted Continuous operation. Small margins are sensitive to cost/yield assumptions. Older schema 2 outputs must be regenerated from their original raw measurements; they are never silently reinterpreted.

Canonical workload rows are alternative scenarios for one planned launch, not simultaneous recipients of repeated 3,000 ICP seeds. Their balances are simulated separately and never pooled.

## Notification and cost decomposition

Global and specific notifications are modelled separately. A completed poll
that processes one or more live blocks produces at most one notification per
global subscriber, even if every block is irrelevant to specific targets. An
empty poll produces none. Specific notifications use two explicit workload
inputs: the fraction of blocks matching any specific subscription and the
assumed number of distinct subscriber matches per matching block. They are
capped at one notification per specific subscriber per poll. A separate
overlap count prevents a subscriber present in both registries from being
charged twice when one notification carries both the global flag and its
independently filtered specific matches. This is a deterministic distribution assumption, not a claim that
every relevant block matches every subscriber.

The batch projector tracks which completed polls actually process live blocks. It collapses repeating per-poll states for long horizons, while a bounded per-poll reference checks burst and backlog cases. Each workload supplies an **assumed** blocks-per-poll capacity (4,096 in the default fixture). PocketIC completing a 4,096-block backlog supports that test size but does not establish a protocol maximum. Archived prefixes skipped without live decoding do not generate notifications in the workload model.

The model calibrates canonical ICP and generic ICRC-3 polling independently. For each available architecture it estimates an idle poll, residual per-block coefficient, and additional-page residual. Notification cost comes from the settled 0-subscriber to largest global-fan-out slope.

Recurring maintenance is a separate event stream rather than part of poll cost. The hourly funding estimate is the maintenance-inclusive Economy median minus the isolated canonical poll coefficient. The daily pricing estimate is the integrated all-lane Fast residual after subtracting isolated polling and the funding estimate. These are qualified residual estimates because the lanes were not individually cycle-metered. Charging them independently avoids counting Economy's coincident hourly poll and funding lane twice.

Entry-to-exit runway and equal-balance comparisons are distinct. The latter give every candidate 75 T, 100 T, 125 T, or 160 T, so raising an entry threshold cannot appear to create resources. Continuous is evaluated at assumed completed-poll intervals of 1, 2, 5, and 10 seconds. Lower-tier objectives are evaluated from each tier's entry to its exit for both canonical and generic expected workloads; ending backlog is reported alongside runway.

The mock Ledgers include the transfer fields Event Horizon decodes and explicit non-transfer blocks, but they do not reproduce mainnet response sizes, replica load, consensus, archive subnet placement, or every encoding mix. PocketIC simulated timestamps test timer semantics. They are not host wall time and are not production latency distributions. Continuous means no deliberate post-poll delay; the model's effective interval is therefore an explicit sensitivity parameter rather than a measurement claim.

## Mainnet comparison plan

No benchmark command interacts with mainnet. Later, an operator may collect low-cost read-only evidence with:

```bash
./tools/scripts/mainnet-observe
```

Compare equivalent windows for cumulative backend cycle-consumption counters, balance deltas, inferred cadence/mode transitions, actual Ledger traffic, subscriber fan-out, Faucet receipt timing, CMC conversion, and ICP/XDR outcomes. Some management-canister counters require controller authority and become unavailable after controller removal; record that limitation rather than adding a production diagnostic API. Mainnet calibration should adjust local projections, not recreate Continuous mode with a large real-ICP top-up.
