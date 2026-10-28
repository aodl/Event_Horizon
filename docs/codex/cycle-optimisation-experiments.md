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
profile 2 was selected in `9e304e1` after all 48 screening pairs passed. Local
screening is not canonical release evidence.

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
periodic301-second storage accounting applied near round50; this fixture uses 200
fixed drain rounds on both variants, then requires 20 quiescent rounds. Initial
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

Two normal integration scenarios now also run the verified canonical81c5a0d
production backend before upgrading to a candidate production module against
fixed trust anchors. Test-only baseline debug seeding/inspection surrounds the
production interval. They cover self/external/global relations, lower thresholds,
permanent aliases, both cursors, pricing, both pending CMC phases with a frozen
surplus, two optimised matching/upgrade passes, and exactly one final surplus.
`EH_TEST_PRODUCTION_WASM` optionally selects final canonical bytes for the rerun.
The original debug81baseline upgrade and44d83c0 historical tests remain.

Frontend compiler screening installs each genuine shrunk production frontend,
verifies20 certified HTTP responses against embedded source bytes, and records
installation/certification debit, allocated memory and status idle burn. Status
query statistics are **reported estimates**, not exact per-request counters:
current [IC abstract behavior](https://docs.internetcomputer.org/references/ic-interface-spec/abstract-behavior/)
specifies aggregation delay and information loss. This fixture reports13calls for
20known successful requests even after several epochs; do not divide those
instruction estimates by20 or claim exact query savings. Response certificate
proof sizes can differ; static asset bytes are checked exactly. The initial
frontend probe used nominal added funding rather than the actual initial balance;
its overflow was a fixture failure and produced no accepted CSV.

Ordinary no-interested-account traffic also covers1/2/16/64pages for recalibrating
cost models without attributing specific-dispatch fees to raw block processing.


The fixed-time fixture additionally has a full24-hour Standard window using
600-second clock steps and20settlement rounds perstep at unchanged clock. This
models negligible message latency within eachstep. It requires144completed polls,
24funding runs, onepricing run and a complete dailyalias refresh. Traffic is four
separately received blocks for the real-receiver case;1,024globals stay idle. Actual
window debit and settled window-plus-explicit-debug-control/drain cost remain
separate; no incoming funding is credited inside either window. All modes use a
fixed 200-round final drain plus 20 quiescent rounds. These day measurements are
separate from660-second latency-conditioned Standard/Continuous fixtures.

Fixed-work CSVs retain actual sender balances before/after the settled interval.
`tools/prepare-cycle-optimisation-model.py` converts complete three-sample rows to
the **existing** cadence model, with fresh per-architecture/registry-size costs and
current policy only. Its zero-traffic, no-incoming-funding projections cover the
ordinary-polling component; hourly/daily maintenance coefficients remain unavailable
there and must not be presented as zero total cost. Actual whole-day scheduler
burn is reported separately. Continuous latency sensitivity is an assumption,
not a measured mainnet poll rate or full-system runway guarantee.

The generated instrumentation archive also has a test-only sender-reserve wrapper.
It compares actual typed/raw CDKget_cost, burns only fresh PocketIC fixture cycles
to reserve+actualcost-1/equal/+1, then calls the exact sender implementation. The
host check requires actual authenticated receiver decoding/digest on accepted
sends, including maximum arbitrary-precision records and cold/warm global bytes.
No production export, liquidity override, retry, execution/decoder limit or stable
region is introduced. These shadow artifacts must never be used as releases.

The largest close-reserve wrapper attempt256×32769 hit baseline's ordinary40B
message budget while decoding and encoding the same value three times. It
produced no accepted CSV and does not establish a production sender limit.
The bounded policy probe uses64×32769 and256×4097; all supplied-balance/cost
boundaries and receiver digests pass. The original full256×32769 receiver
regression stays unchanged. No execution/decoder limit or production hint count
was changed. Source4cd89c8 reproduces the rejected wrapper envelope.

The `mixed:1:1024` fixed-work case seeds512global-only and512specific synthetic
recipients under disjoint identities. It requires1024attempts and accepted sends,
records exact aggregate ordinary typed payload bytes, and exercises shared global
encoding alongside distinct specific encodings. Separate real mixed-record/Index
and scheduler fixtures verify actual receiver contents and authentication.


Final settlement correction: an unchanged generic 16-page/512-relation fixture
repeatedly applied a 12,258,255-cycle debit near round 50 after advancing the
clock by 301 seconds. Its old 40+10 check failed; it was never accepted as a
complete row. A diagnostic 200+20-round run passed with a flat balance after
round 50. Fixed-work, publication, small, production and scheduler final-drain
checks now use 200+20. Earlier partial matrix costs remain provisional and are
superseded by a new input receipt and complete rerun, not mixed into final medians.

Two normal ignored state-differential tests read a copy of each canister's memory
with the existing pinned `ic-stable-structures` SDK. They compare every logical
key/value/cell in IDs 0–11 at three checkpoints, never physical B-tree page bytes.
The SDK dependency is a native test-only edge in Cargo.lock; no package version or
production graph changed. Financial comparisons supply the same liquid balance
at the funding decision: 150T is below both actual debug-setter samples, so the
unchanged surplus observation records the same minimum. Every stored financial
field remains part of the comparison; final balances are not asserted equal.


### Reproducing individual changes and attribution

The focused matrices are retained in executable form:
`python3 tools/run-cycle-optimisation-components.py` and
`python3 tools/run-cycle-optimisation-attribution.py`, from the repository root.
They use the documented `observations/cycle-optimisation/` artifact layout, normal
frozen mocks and official PocketIC15. Build every input before starting either
serial runner. The component runner uses three independent pairs per row; the
attribution runner uses one deterministic instrumented pair and cannot establish
headline cycle savings.

| Artifact directory | Source commit | Build/transform |
| --- | --- | --- |
| baseline fixture | `81c5a0d` | pinned release z, debug_api, no shrink |
| ab | `cdc555b` | same |
| c | `cc5ce45` | same |
| d | `5ddbbc9` | same |
| e | `da43560` | same |
| f | `7cc440e` | same |
| g | `7425e22` | same |
| i | `57b967b` | same |
| j | `00c0d63` | same |

Create a separate clean worktree for each revision, set a distinct
`CARGO_TARGET_DIR`, then run
`cargo build --locked --target wasm32-unknown-unknown --release -p event-horizon --features debug_api`.
Copy its raw module once to `ARTIFACT_DIRECTORY/event_horizon_debug.wasm` and hash
it; do not reuse a target directory across relocated source archives. These local
prototypes isolate source changes and are separate from the final-shrunk compiler
screen and final canonical production artifacts. The two ICRC key-capacity
comparisons use i→j; their complete raw rows are retained separately.

The final uninstrumented matrix uses baseline/candidate shrunk companions from
`tools/screen-cycle-optimisation-profiles.py`, profiles z and2 respectively, plus
frozen mock binaries. For shadows, generate independent baseline and candidate
archives with `tools/instrument-cycle-optimisation.py`; build baseline-z,
candidate-z and candidate-2 using pinned flags and distinct target directories.
For each source/profile preserve both production `optimisation_metrics` and debug
`debug_api,optimisation_metrics` builds, shrink with0.9.7, and name them
`final-shadows/VARIANT/event_horizon_production.wasm` and
`final-shadows/VARIANT/event_horizon_debug.wasm`. The attribution script's cases
and synchronous scope definitions are tracked, and artifact/input digests in the
data receipts identify the actual modules measured.


Current primary [cycle-cost guidance](https://docs.internetcomputer.org/references/cycle-costs/)
and [charging rules](https://docs.internetcomputer.org/concepts/cycles/), rechecked
2026-10-28, distinguish uncharged non-replicated client queries from charged
replicated execution and messaging. Observation queries do not establish a frontend
query-cycle saving; frontend comparisons report code/storage/install costs and
coarse instruction statistics separately. Sender inter-canister work remains in
measured debits. Memory billing is periodic, which is why a same-clock quiescence
check after the declared clock advance is required instead of an immediate balance.

The fixed-time scheduler fixtures use configured-debug runtime records and actual
unchanged scheduler lanes. Their telemetry/control are test-only. They omit the
production runtime's fixed-principal parsing path and are not canonical production
cost measurements. Existing-model forecasts use explicitly labelled debug polling
coefficients; maintenance is unavailable there and no total mainnet runway is
claimed. Genuine production Standard-timer comparisons separately exercise the
fixed trust anchors. Final canonical shared/fan-out/page scenarios remain required.

### Final real-receiver production matrix

The feature-only `production_real_fixed_work` fixture seeds relations through real
ICP/Historian admissions before upgrading to each genuine production module. It
uses fixed trust anchors, ordinary execution limits and 200+20-round settlement.
Both variants receive80Tactual cycles, use unchanged VeryFast cadence and advance
10seconds for one measured poll. This covers simultaneous response/callback
reservations (42,102,481,000cycles for the65-byte global call in the pinned setup).
The original6Tfixture legitimately skipped140of256calls under reserve policy and
changed toEconomy; it was rejected, not an efficiency result. Seed admissions use
16payout completed batches to respect PocketIC's100round ingress wait. Setup cost
is excluded. The older seven-case production matrix stays6T/Standard/600seconds;
do not mix these cohort windows. Dependency/recipient debits include their own
declared storage interval and are not pure execution costs.
The predeclared matrix, for both architectures and three independent pairs, is:
32 real receivers with 256 equal/increasing transfers; 32 admitted but unmatched
receivers with increasing transfers; 256 real global receivers; and 512 relations
with 4,096 blocks, retention overflow and a final update of retained target zero.
Every intended receiver must authenticate and decode exactly one complete record.
The unmatched case verifies its effective thresholds independently in stable ID11.
Setup, inspection upgrades and receiver/dependency costs are outside sender debit.
An explicitly selected 1,024-real-receiver smoke is separate from headline samples.

Run `tools/run-cycle-optimisation.py production-real BASELINE CANDIDATE OUTPUT
--mocks FROZEN_MOCKS` with the same environment as the production command above.
Final evidence requires `EH_OPT_CANONICAL_SOURCE=$(git rev-parse HEAD)`, a clean
source tree and candidate path `release-artifacts/event_horizon.wasm`; an arbitrary
local file is not sufficient. This host extension changes no production code.

### Scheduler boundary and model qualifications

The ICP one-real-subscriber day has a raw boundary debit increase of 2,617,501
cycles (0.0363%), despite a settled window plus control/drain reduction of
46,187,145 (0.6356%). Both are retained. Optional `EH_OPT_DIAG_LANES=1` prints
nonreplicated lane observations after capture, without an extra tick or ingress,
to investigate outstanding funding/pricing reservations at that boundary.

The existing cadence model requires workload IDs `expected`/`expected_generic`;
the adapter uses those IDs for explicitly idle ICP/generic workloads. An earlier
adapter failed this validation and produced no accepted simulation. Four corrected
sets each run 63 simulations using fresh three-sample median coefficients, current
policy and seed 20261028. No maintenance coefficients are invented. These are
ordinary-polling projections, not whole-system burn or mainnet runway.

For final frontend bytes after pinning, select only the validated release level
with `EH_OPT_FRONTEND_LEVEL=2` in `frontend_profile_work`, placing the canonical
frontend at `EH_OPT_FRONTEND_PROFILES/2/event_horizon_frontend.wasm`. This performs
20 source-asset/certification-response checks without rebuilding other compiler
levels. Old pre-pin compiler fixtures remain bound to their original source;
they must not be compared to newly pinned source assets.

Unimplemented repeat-idle/bootstrap/archive labels were removed only from the
old production helper's selectable list; none had accepted production results.
Their fully implemented debug scenarios remain intact. All CSV-producing host
fixtures reject an empty selection before writing evidence. No regression
scenario was removed or delivery assertion relaxed.


Scenario names are literal: `backlog` repeatedly transfers amount20 to the same
watchedtarget7 with16watchers on everypage. `growing` admits512targets, retains
first256, processesmostlyunwatched targetnumbers after512, and updates retained
zero in the lastblock. `ordinary` has no watchers. These distinguish repeated
matching, full-accumulator/irrelevant trailingpages and uninterested scan costs.
