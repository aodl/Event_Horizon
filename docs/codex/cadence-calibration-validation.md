# Cadence calibration validation

## Scope and provenance

This validation uses the retained PocketIC evidence from the reviewed baseline
`1d04faf9d0df58b551f54323fd651b57ab147440`. Schema 3 corrects measurement
self-copy handling, queue interpretation and financial classification. It
supersedes the previous simulator interpretation, while retaining the original
raw measurements and the earlier notification/coalescing corrections. The
125 T candidate remains an experimental comparison.
Production thresholds, polling delays, funding behaviour, notification
semantics, stable memory, Candid, and immutable configuration remain
unchanged. No mainnet call, deployment, reinstall, upgrade, top-up, ICP/cycles
transfer, or controller operation was performed.

The final review output is locally available at
`observations/cadence/review-final/` and is intentionally ignored.
Its `measurements.json` and `measurements.csv` are byte-identical copies of the
previously completed full measurement dataset:

- JSON SHA-256
  `d7ec1ed90b8390188df62dd8cab882705ad2353e1b5e8ec006aba5c12f505797`;
- CSV SHA-256
  `92942cc93b53f1a54355678857c8e4ff25082abd25412d6cea98604df0620eb9`;
- source revision `41e088f471e807e202f54b53009d1d56e37910ba`, with review changes recorded
  as a dirty working tree;
- PocketIC 15.0.0, one application subnet, normal cost schedule, and seed
  `20261011`;
- debug/test backend Wasm SHA-256
  `6f20cf491802d35555d4a76cffddc31d56147232f4cb257d7e5ae947c3be3572`;
- 54 complete raw scenarios and 50 threshold observations, with no cap,
  truncation, or omitted measurement failure; and
- 205 final deterministic simulations from the complete dataset.

Final output hashes and gate results are recorded below. The full raw
measurement matrix was not rerun: no empirical cost coefficient changed. A
fresh smoke run completed nine scenarios and all 50 threshold observations at
`observations/cadence/smoke-final/`. All 205 adaptive scenarios and 40 separate
fixed-tier economic comparisons were generated from the retained full dataset.

## Measurement integrity

The former CSV copy could open its own input for writing; the JSON guard only
compared textual paths. Both copies now resolve existing canonical paths and,
on Unix, device/inode identity before copying. Identical files are skipped,
including directory/file symlinks and hard links. Separate destinations remain
supported. Resolution/copy errors include paths and fail the command; unexpected
CSV metadata errors are no longer treated as an absent optional CSV.

Regression tests check JSON/CSV SHA-256 and modification times through in-place
and repeated model/report runs, relative/absolute dot-component paths,
directory and destination-file symlinks, hard links, separate output copies,
and a destination directory that causes a copy failure. A disposable copy of
the full raw measurements was also modelled in place. The original dataset
was never used for destructive-path experiments and retains both hashes above.

## Corrected notification model

Global and specific notification counts are now independent:

- every nonempty completed poll notifies each global subscriber once,
  regardless of whether a block matches a specific target;
- an empty poll sends no global notification;
- specific notifications use explicit matching-block and distinct-subscriber
  distribution assumptions and remain capped once per subscriber per poll;
- a subscriber in both registries is charged once when a specific payload
  takes precedence over the global empty hint;
- redundant matches in one poll are coalesced; and
- recipient execution remains outside Event Horizon's burn.

The final batch correction counts polls that actually process live activity,
including the sequence of backlog draining and subsequent empty polls. A
1,000-block backlog that fits in one poll sends one global poke across 100
polls, rather than the former aggregate estimate of 100. Tests compare the
cycle-skipping batch projector with an independent bounded per-poll reference,
including capacity-limited bursts and sustained traffic. The default 4,096
blocks-per-poll capacity is an explicit modelling assumption; completing a
4,096-block PocketIC case does not prove a hard protocol ceiling. Focused
tests also cover one block per poll, irrelevant-only activity, empty polls,
ten blocks coalesced into one global poke, disjoint specific targets, and mixed
global/specific delivery. The expected workload assumes one block per second,
5% specifically matching blocks, one distinct specific subscriber per
matching block, one global subscriber, and five specific subscribers. It does
not assume that every matching transaction wakes all five subscribers.

The empirical notification coefficient remains 5,488,565 backend cycles per
additional dispatch. The underlying 0/1/10/100/256/1,024-subscriber
measurements were not changed.

## Funding consistency

The expected workload has productive backing for one global declaration,
three account declarations, and two range declarations. The global reference
basis is 1,000 ICP; account and range bases remain 10 and 20 ICP. The
high-activity multiplier was corrected from an impossible 125% to 100%.
Multipliers must lie in 1..=10,000 basis points, and each admission class is
rounded independently to whole ICP: at 62.5%, the three minima are 7, 13, and
625 ICP. Patrons can voluntarily endow more than the qualifying minimum.
The default funding calculation assumes exact-minimum patron endowments; extra
principal is not represented by an inadmissible multiplier above 100%.

The planned 3,000 ICP productive protocol endowment is now assigned to each
canonical workload; the independent generic instance has no protocol seed and
receives payouts only from its own subscriber endowments. The stress workload
decouples subscriber counts from subscription backing but retains the
canonical protocol seed. Productive principal is never credited as liquid
cycles. Only assumed maturity-derived Faucet payouts enter each instance's
modelled balance.

## Cost profiles and maintenance

The corrected model retains separate reader profiles:

| Architecture | Empty poll | Residual block | Additional page |
|---|---:|---:|---:|
| Canonical ICP shared scan | 22,069,775 | 209,200 | 6,624,846 |
| Generic ICRC-3 plus ICP admission | 28,238,603 | 268,939 | 6,188,199 |

Page, block, decoding, and state costs remain partially confounded. These are
PocketIC coefficients, not universal replica prices.

Recurring maintenance is charged as independent scheduled work and never
folded into ordinary polls:

- hourly funding maintenance: 21,220,513 cycles/run, estimated from the
  Economy maintenance-inclusive median minus the isolated poll coefficient;
- daily pricing maintenance: 16,619,093 cycles/run, estimated from the
  integrated Fast all-lane residual after removing polling and funding.

These are qualified residual estimates rather than independently isolated
lane measurements. Over 365 days they contribute approximately 0.191 T cycles
in the corrected expected-workload runs.

## Corrected results

At each policy's own Continuous entry, no funding, and the expected workload:

| Policy | Entry/exit | 1 s | 2 s | 5 s | 10 s |
|---|---:|---:|---:|---:|---:|
| Current | 100/60 T | 16.51 d | 32.45 d | 77.14 d | 142.65 d |
| Former 125 T candidate | 125/60 T | 26.82 d | 52.72 d | 125.36 d | 231.80 d |
| Conservative | 160/90 T | 28.89 d | 56.78 d | 135.00 d | 249.63 d |
| Performance-oriented | 75/45 T | 12.38 d | 24.33 d | 57.86 d | 106.98 d |

No candidate meets the provisional 30-day Continuous objective at the
one-second assumption. At two seconds the current policy already exceeds 30
days. Because the effective production completion rate is not yet measured,
the local evidence cannot justify choosing 125 T or another replacement.

## Window coverage, liquidity and mature economics

Schema 3 removes `sustainable`, whose old definition combined liquidity and
finite-window receipts. `window_funding_covers_expenditure` is the arithmetic
receipt/spend comparison, independently of liquidity. Minimum/ending cycles,
reserve entry, exhaustion, time in each mode and suspended seconds describe
safety. Backlog is excluded from the liquidity classifier.

The expected canonical principal remains 4,070 ICP: planned protocol backing
3,000 ICP, one global 1,000 ICP, three accounts 30 ICP and two ranges 40 ICP.
The illustrative effective 5.6% APY yields 227.92 ICP or 455.84 T annually before
conversion fees at 2 XDR/ICP. The checked integer calculation annualizes the
fee-net weekly payout rate to **455.829571100000 T/year**. These inputs are
assumptions, not confirmed principal, guaranteed income or measured Faucet yield.

The fixed-tier rows exclude startup backlog and assume a continuing workload
with independent hourly funding and daily pricing maintenance. They include
empty polls, processed blocks, additional pages and coalesced global/specific
notifications. They are separate from the actual adaptive path. Annual processed/generated counts also identify capacity-limited fixed-tier rows: high-activity Economy and stress Economy/Standard do not process all arrivals. A positive financial margin in those rows does not establish adequate service.

| Continuous completed-poll interval | Fixed annual expenditure (T) | Mature annual margin (T) |
|---|---:|---:|
| 1 second | 884.523468394825 | -428.693897294825 |
| 2 seconds | 449.983563274825 | +5.846007825175 |
| 5 seconds | 189.259620202825 | +266.569950897175 |
| 10 seconds | 102.351639178825 | +353.477931921175 |

The two-second mature margin is about 1.3% of expenditure. It does not prove
indefinite Continuous operation or sufficient bootstrap liquidity. Fixed
Economy, Standard and Fast expected-workload annual costs are respectively
8.083570181665, 10.376553906745 and 21.243595024825 T; affordability depends
on the selected continuing tier, traffic and fan-out assumptions.

Regular payouts now follow a completed accrual period, rather than paying
one period in advance at recognition. The fixture retains its illustrative
14-day recognition delay plus one-hour conversion delay, weekly cadence and
10,000-e8s conversion fee; this is a model input, not a change to Jupiter's
reviewed seven-day recognition policy. A delayed lump sum retains the existing
per-delivery fee semantics. Startup delay changes receipts, not principal or
the mature recurring rate.

| Current policy, weekly funding, 365 days | Receipts (T) | Adaptive expenditure (T) | Window surplus / deficit (T) | Minimum cycles (T) |
|---|---:|---:|---:|---:|
| 1-second assumption | 437.096849000000 | 469.567284777780 | -32.470435777780 | 58.728159934480 |
| 2-second assumption | 437.096849000000 | 449.983529809520 | -12.886680809520 | 74.059222107300 |
| 5-second assumption | 437.096849000000 | 189.259586109920 | +247.837262890080 | 89.089541370900 |
| 10-second assumption | 437.096849000000 | 102.351604039920 | +334.745244960080 | 94.099648628900 |

All four maintain liquidity without reserve entry or exhaustion. At two
seconds, a first-year deficit coexists with positive mature economics. At one
second, adaptive downgrades reduce actual expenditure far below the hypothetical
fixed Continuous rate; beginning in Continuous does not imply remaining there.
Small phase differences between annual fixed rates and adaptive expenditure
reflect maintenance/poll ordering at the horizon boundary.

Before this correction, weekly first-year receipts were 445.838785980000 T.
Paying completed periods reduces them by one weekly receipt, 8.741936980000 T.
At two seconds, projected spend remains 449.983529809520 T and window coverage
remains negative, but the former combined classification concealed the positive
mature margin. At one second, later receipts change the adaptive path: spend
falls from 479.353830873720 to 469.567284777780 T. No-funding expenditure,
runway and ending backlog remain unchanged; the new queue metrics correct
interpretation and include previously unsampled peaks.

Equal-balance comparisons start every policy at 75, 100, 125, and 160 T. At
100 T, for example, the current policy immediately enters Continuous and
spends 16.51 of the 90 days there; both the 125 T and conservative candidates
remain Very Fast for the full 90 days. The performance-oriented candidate
spends 22.70 days in Continuous but retains less balance. Raising an entry
threshold therefore trades responsiveness for protection; it does not create
runway from the same resources.

The current lower-tier entry-to-exit results for the expected workload are:

| Tier | Canonical runway | Generic runway | 30-day objective |
|---|---:|---:|---|
| Economy | 45.17 d | 36.62 d | Both meet |
| Standard | 70.35 d | 58.22 d | Both meet |
| Fast | 68.73 d | 58.99 d | Both meet |
| Very Fast | 35.66 d | 29.51 d | Generic misses narrowly |

Economy, Standard and Fast clear their normal between-poll arrivals at one
block/second under the configured 4,096-block capacity. The projector retains
maximum pre-poll waiting and maximum post-poll outstanding blocks across each
batch, plus first-downgrade/reserve snapshots and arrivals during reserve episodes.

| Starting tier, canonical no funding | Whole active-path pre-poll maximum | Post-poll maximum | First downgrade / reserve outstanding | Reserve arrivals = ending backlog |
|---|---:|---:|---:|---:|
| Economy | 3,600 | 0 | 0 / 0 | 27,633,600 |
| Standard | 3,600 | 0 | 0 / 0 | 17,652,600 |
| Fast | 3,600 | 0 | 0 / 0 | 8,675,520 |

The whole-path maxima include later Economy operation. Before the first
downgrade, Standard waits up to 600 blocks and Fast up to 120; both clear each
poll. The large ending queues above arise entirely after reserve suspension,
not active throughput incapacity. Very Fast's no-funding path ends with 800
waiting blocks without reserve entry or post-poll lag. Excess-capacity scenarios
instead show persistent post-poll growth. Recovery can drain reserve arrivals;
that cumulative metric is not the remaining queue. Backlog does not prove
archived records were missed. Live retention and explicit archive evidence
remain limitations of the best-effort reader, with no archive traversal added.
These service and architecture results do not justify changing lower thresholds.

Earlier removal of the generic instance's incorrectly inherited protocol seed
and correction of the high-activity admission multiplier remain in force.
They are separate corrections from this pass. The raw PocketIC coefficients,
scenario provenance and no-funding runway table above are unchanged.

Interruption comparisons reached no Reserve Protection event from their own
candidate entry balances, but produced materially different minima and
responsiveness. Stress workloads show window deficits and large queues for every candidate;
active lag and reserve arrivals must be assessed separately. Rolling seven-day transition counts remained
bounded; the raw 50-case PocketIC hysteresis parity result is unchanged.

## Conclusion

The corrected model does not identify a static candidate that dominates the
current policy across poll-rate sensitivity, equal-balance responsiveness,
funding interruptions, backlog safety, reserve outcomes, and hysteresis.

The evidence-based recommendation is therefore **no production threshold
change yet**. This is a provisional retention decision, not proof that the
current table is optimal. Mainnet observation of the effective Continuous
completion rate, actual cycle burn, reader response sizes, traffic, fan-out,
and Faucet receipts is required before reconsidering the thresholds.

## Reproduction

```bash
./tools/scripts/calibrate-cadence smoke --seed 20261011 \
  --output observations/cadence/smoke-final
./tools/scripts/calibrate-cadence model \
  --input observations/cadence/full-20261011/measurements.json \
  --config tests/cadence-calibration/fixtures/model-config.json \
  --output observations/cadence/review-final \
  --seed 20261011
./tools/scripts/calibrate-cadence report \
  --input observations/cadence/review-final
```

## Dependency security and production isolation

Targeted updates resolve the nine reviewed actionable findings:
`RUSTSEC-2026-0338` through `RUSTSEC-2026-0342` in `quinn-proto` (0.11.15 →
0.11.18), and `RUSTSEC-2026-0346` through `RUSTSEC-2026-0349` in `rustls`
(0.23.45 → 0.23.46). Quinn requires rand 0.10: rand 0.9.3 → 0.10.3,
rand_core 0.9.5 → 0.10.1, new rand_pcg 0.10.2, chacha20 0.10.2 and
cpufeatures 0.3.1. getrandom 0.3.4, r-efi 5.3.0 and rand_chacha 0.9.0
become unnecessary; existing getrandom 0.4.2 gains the corresponding features.
PocketIC remains pinned at 15.0.0. No broad update or new exception was used.

Cargo metadata closure traversal and Wasm-target normal/build trees show no
changed package in either production canister closure. Rustls is used through
PocketIC → reqwest → hyper-rustls/tokio-rustls. Quinn's optional HTTP/3 closure
is locked but not enabled in the current native build, so the ordinary reverse
tree reports nothing for it; the published reqwest dependency declaration and
lockfile identify its origin. These are test-tooling changes. Production source,
interfaces, configuration, frontend registry and pricing policy are unchanged.

`security-scan` passes cargo-audit, cargo-deny advisories/licenses/bans/sources,
npm audit and OSV with zero unfiltered findings. The existing maintenance-only
exceptions remain separately identifiable: RUSTSEC-2025-0012 (backoff),
RUSTSEC-2024-0384 (instant), RUSTSEC-2024-0436 (paste), and RUSTSEC-2021-0127
(serde_cbor). Scan scripts, workflow and allowlists are unchanged.

## Validation gates

Completed local evidence:

- `cargo test --locked -p event-horizon-cadence-calibration`: 31 tests passed;
- smoke: nine complete scenarios and 50 threshold observations;
- full offline output: 205 unique adaptive scenarios and 40 fixed-tier rows;
- all horizon sums, expenditure components, margins, coverage and liquidity
  classifications checked for internal consistency;
- disposable full-dataset in-place and directory-symlink runs: both measurement
  hashes unchanged (a narrowed zero-traffic config isolates file handling);
- formatting, Clippy and diff checks pass;
- security scan passes with zero unfiltered findings and four named existing
  maintenance exceptions;
- final `xtask test_unit`: five suites, 149 passed tests (119 Rust and 30 frontend), including 31 calibration tests; static validation, canonical-context fixture checks and source manifest pass;
- final `POCKET_IC_MUTE_SERVER=1 DFX_IDENTITY=codex_local xtask test_all`: 111 suites, 255 passed tests, including all 106 fresh PocketIC scenarios; zero failures.

The model/report commands also ran directly through the same CLI after a locked
build, avoiding long-held Cargo build locks during repeated offline simulations.
The opt-in wrapper is unchanged. Repeated final model/report output hashes are
checked byte for byte, not merely by comparing rounded report values.

Final research output SHA-256 values:

| Output in `observations/cadence/review-final/` | SHA-256 |
|---|---|
| `measurements.json` | `d7ec1ed90b8390188df62dd8cab882705ad2353e1b5e8ec006aba5c12f505797` |
| `measurements.csv` | `92942cc93b53f1a54355678857c8e4ff25082abd25412d6cea98604df0620eb9` |
| `simulations.json` | `34655a6bd443c280b5e3ecdd1c370398669d6642e211ab9ac6924c2e9b5808a3` |
| `threshold-comparison.csv` | `b0accbaef77cf7ebf11ff30b11af967b878c0b6dacd86f81b5118c8e7880d29b` |
| `report.md` | `f0b23dc75c5d9bbe4b75ec91ec10aed19d7b6c84d4ccfbddbc9927b519802cc4` |

Repository hygiene found no redundant tracked file requiring removal. The
compact fixture, opt-in runner, separate methodology/validation documents and
historical provenance are retained. Temporary outputs from interrupted file
regressions are removed; generated research remains ignored. Both production
normal/build dependency closures are unchanged, and source-manifest membership
covers the final tracked source.
The remote Dependency Security run remains pending: no push is authorized.
Canonical two-build reproducibility, artifact hashes, manifests and production
export audits are checked from the clean focused commit after these source and
documentation changes are committed; those post-commit results accompany the
handover rather than altering its already-validated commit.
