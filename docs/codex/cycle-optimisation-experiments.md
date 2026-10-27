# Cycle optimisation reproduction and measurement definitions

The independent implementation baseline is `81c5a0d72ff188f5ee9f412deb20566a12307642`.
Its debug and canonical production companions and SHA-256 provenance are under
`tests/fixtures/`. Runtime prototypes are separate reviewable commits A/B through G;
see [resumable progress](cycle-optimisation-progress.md). They are not yet a validated release.

Use Rust 1.94.1, the locked dependency graph, PocketIC 15.0.0 ordinary execution
limits, and `DFX_IDENTITY=codex_local`. Run builds before measurements, and measure
pairs serially. Each new baseline/candidate instance has identical fixture IDs,
responses, simulated timestamps, funding and settlement criteria. Never replace
baseline bytes with a candidate build. Costs are settled sender balances; mock and
recipient costs, fixture setup and observation queries are outside the interval.
Debug synthetic recipients measure accepted send attempts, not remote delivery.
Status memory is settled total/stable allocation, not peak live heap. The production
Standard-timer interval includes identical 600-second storage accounting. No
subtraction of instruction counters spans an await.

Build a debug candidate once, copy it to an immutable named observation directory,
and record its digest. The paired runner accepts explicit paths, creates fresh
processes per complete pair, requires three independent samples per case, refuses
changed resume inputs and excludes failed/incomplete shards:

```bash
python3 tools/run-cycle-optimisation.py debug \
  tests/fixtures/event_horizon_81c5a0d_debug.wasm \
  observations/cycle-optimisation/g/event_horizon_debug.wasm \
  observations/cycle-optimisation/combined-debug
python3 tools/summarize-cycle-optimisation.py \
  observations/cycle-optimisation/combined-debug/complete.csv
```

Use `--case equal:256:1024 --generic false` for a focused comparison; still three
samples. The default matrix includes idle/global 0/1/256/1,024 recipients, shared
account amounts (equal/increasing/decreasing/alternating), unique destinations and
1/2/16/64-page backlogs. Additional admission/alias/huge-amount evidence is required
before final acceptance; this default alone does not satisfy the full task matrix.
Input receipts retain source, harness, artifact and lock identities; raw pair logs
and CSVs remain usable after an interrupted run. `complete.csv` is produced only
when every requested shard passes. These are fixed-work costs, not daily-burn
projections or physical message-order guarantees.

For production-path smoke/comparisons, use runner mode `production` and explicit
production companions. Fixed trust-anchor mocks are installed without changing the
canister's production principals or exports. Verified baseline debug bytes seed
state; argument-free upgrades switch to genuine production execution; a later
inspection upgrade occurs only after sender costs/notifications are captured.
Canonical claims additionally require clean committed source and supported Docker
provenance, not merely providing a file to the runner.

For instruction attribution, generate two separate archives with
`python3 tools/instrument-cycle-optimisation.py REVISION NEW_DIRECTORY`, then build
each with `--features optimisation_metrics` (never `debug_api`). Use **distinct
target directories**: Cargo can otherwise reuse freshness from a relocated
workspace with the same relative paths. Preserve each artifact and its receipt.
Run production mode with `--metrics`; metric scopes cover synchronous operations
only. Nested scopes overlap and must not be summed. Instruction counts include
counter overhead; compare instrumented cycles with uninstrumented rows, never
substitute them for headline costs. Maximum observed linear-memory pages is an
allocation high-water at sampled scopes, not a live-heap profiler. Shadow exports
are test-only and must be absent from canonical Wasm.

Compiler screening is predeclared before its first run: baseline `z`, candidate
`z/s/2/3`; production ordinary traffic/global-only real receiver plus debug idle
1,024 globals, equal and increasing 256×1,024 watchers, and unique destinations.
`tools/screen-cycle-optimisation-profiles.py` requires a clean source worktree and
pinned `ic-wasm 0.9.7`, preserves raw/shrunk artifacts and audits production exports.
Toolchain, locks, LTO, codegen units, panic, overflow and target settings stay fixed.
The winner needs full behavioural/performance gates, including adverse workloads;
no profile is selected yet. Local screening is not canonical release evidence.

Final reporting must distinguish fixed-work savings from unchanged-scheduler
fixed-time/funding measurements and recalibrated model projections. Continuous
mode can buy more polls with lower cost. No short fixture establishes mainnet
latency, daily burn or runway guarantees.
