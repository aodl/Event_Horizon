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
samples. The default matrix includes idle/global 0/1/256/1,024 recipients, first bootstrap,
eight repeated empty polls, archive-only cursor progress, and shared
account amounts (equal/increasing/decreasing/alternating), unique destinations and
1/2/16/64-page backlogs. Publication mode separately measures no-op/new admissions, monotonic reductions,
full-width ranges and unchanged/additive 2/4,096-entry alias tables. Generic huge
amounts are separate rows. The 32,769-bit ×1,024-recipient baseline exceeded normal
40B message instructions and is preserved as a failed boundary; its accepted
stress envelope is 16 recipients, with no production limit change. Small mixed
watcher and scheduler evaluations remain separate acceptance work.
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


Freeze normal `z` mock Wasm once before compiler comparisons; provide its directory
with runner `--mocks DIRECTORY`. The host fixture loader uses those exact bytes for
all paired/compiler runs, except the explicitly requested controlled-await mock
wrapper. Inputs retain every mock binary/source digest and PocketIC binary digest.
Use official PocketIC 15.0.0 (`POCKET_IC_BIN`); this session's executable SHA-256 is
`29472ea4433b30a280676c4e22e369d79d5ba6ee1b4d48bab32ebe7d0ad2b4bb`, downloaded from
`https://github.com/dfinity/pocketic/releases/download/15.0.0/pocket-ic-x86_64-linux.gz`.
Normal execution limits are retained; fixture topology/configuration is logged.

Raw heap-before/after fields are allocated Wasm linear memory, a module-lifetime
allocation high-water, rather than live heap. Setup allocation is reported before
the cost interval. Stable allocation and total memory remain separate columns.
Bootstrap/archive cursor progress is distinct from `processed_blocks`; repeated
empty rows explicitly count eight completed polls. No failed/incomplete scenario
becomes an accepted row. Alias-query mock state is discarded: positive alias call
counts come only from caller-side instrumented shadows. Uninstrumented publication
rows leave `alias_calls` unavailable and derive `validated_alias_pages` from a
successful complete snapshot plus its timestamp/revision digest checks. Alias
measurements use a same-variant argument-free debug upgrade to clear scheduled
timers, advance to the next eligible day, and settle setup/storage costs first.
Publication attribution shadows use `debug_api,optimisation_metrics`; their debug
configuration is separate from production-only `optimisation_metrics` measurements.


`small` runner mode installs1/16 real self/external/neuron relations and verifies
complete notifications at authenticated record receivers. Debug state's
`subscriptions` counts only self-map entries; external relations are independently
proved by actual subject-aware payloads. The real-receiver settlement trace found
periodic301-second storage accounting applied near round50; this fixture uses120
fixed drain rounds on both variants, then requires ten quiescent rounds. Initial
failed probes are retained and produced no accepted CSV.

`scheduler` mode evaluates actual unchanged scheduler lanes for660 simulated
seconds at6T (Standard) and110T (Continuous), with1 real mixed subscriber or1,024
idle globals. One explicit PIC tick per simulated second, plus fixture-ingress
rounds, is a stated latency assumption. Setup/credits occur before the window.
Window debits and settled window-plus-debug-control/drain debits are separate;
in-flight work and extra drain lane starts are reported. This is not a daily
mainnet-burn guarantee. A daily projection must be labelled modelled and account
for the drain and memory/storage costs. No production scheduler control is added.

Shadow attribution also supports `debug --metrics`; identical generated scopes
measure synchronous page/matching/dispatch phases and actual invocation counts.
Global count/write/encoding operation proofs remain native-only counters. Original
release overflow-check policy is the unchanged Cargo release default (no explicit
override), not an enabled overflow-check flag.
