# Cycle optimisation data

The immutable final canonical source is
`7cfa8b354fa9621786a09b13c9a6dff498872b19`; the independent baseline is
`81c5a0d72ff188f5ee9f412deb20566a12307642`. See the
[validation report](../cycle-optimisation-validation.md) for release blockers and
[reproduction guide](../cycle-optimisation-experiments.md) for commands and fixtures.

## Final canonical evidence

`canonical-validation.json` records two independent uncached supported Docker
builds and populated-artifact equality, exact hashes/sizes, source manifest and
command/log digests. `canonical-build-info.json` and
`canonical-release-artifacts.sha256` retain the build metadata. No local compiler
experiment substitutes for this evidence.

`canonical-production*` contains 42 standard pairs, while
`canonical-production-real*` contains 30 real-receiver pairs: three independent
samples per group, with raw before/after balances, excluded setup credits, inputs,
summary medians/ranges and accepted-shard/log receipts. The former uses 6T,
Standard and 600 seconds; the latter 80T, VeryFast and 10 seconds. Both run genuine
production modules at unchanged trust anchors. Compare within each cohort.
Failed or incomplete scenarios cannot produce accepted rows.

`canonical-shadow-*` retains eight one-sample diagnostic pairs, synchronous
operation/instruction metrics and the matched uninstrumented-median overhead.
These generated modules are separate from production artifacts. Nested instruction
spans overlap; no counter subtraction crosses an await. The diagnostic costs do
not replace uninstrumented headline measurements.

`canonical-checks.json` retains final command/log receipts, including two genuine
81-baseline production upgrades and 20 certified frontend response checks.
`canonical-frontend.csv` records that final frontend fixture.
`canonical-environment-failures.json` preserves four PocketIC HTTP
transport failures and identical-input passing reruns. Native stderr and host RSS
were captured in separate diagnostic attempts; no transport cause was established. Failed attempts are
not accepted measurements or failed behavioural assertions.
`receipt.json` binds the final evidence files. Large raw logs remain in the ignored
observations directory; their SHA-256 receipts are tracked here.

`historical-sanity.*` explains the reproduced approximately 5.958/7.834B ICP
immediate shared-account costs and separate-ledger global fan-out baseline.
Immediate and settled storage-window debits are distinct. Historical values are
sanity references, never substitutes for measured comparisons.

## Earlier component and configured-debug evidence

Compiler screening uses runtime `00c0d63` and host `8802517`. All 48 independent
z/s/2/3 pairs pass, with one screening sample per row. Profile 2 is selected:
increasing/unique cases slightly favour it over 3, whose other small wins cost an
additional 1.27% backend / 1.54% frontend bytes. Against refactored z, profile 2
adds 29.03% / 6.71%. Screening frontends carry the earlier backend pin; final
canonical pin/artifacts above are separate. Twenty HTTP checks per profile pass;
status query statistics report 13 calls for 20 known requests, so those estimates
do not establish exact per-request instruction savings.

The final-v2 uninstrumented fixed-work matrix is complete: 93 groups × three
independent pairs (279 pairs/558 raw rows). `fixed-work.csv` retains all fields,
`fixed-work-summary.csv` gives medians/ranges for each group, and the input/receipt
JSON binds artifacts, host harness, mocks, lock/tool versions and raw-log digests.
All overall candidate rows improve; ICP reductions range1.6429–36.7097%, generic
1.2871–34.8588%. Zero observed ranges reflect deterministic PocketIC fixtures,
not mainnet uncertainty. Stable allocation is100,728,832bytes for both; largest
allocated Wasm linear memory is3,735,552bytes in either variant. Per-change regressions and publication/scheduler/canonical data are separate.
These are configured-debug fixed-work costs with declared settlement intervals,
not canonical production or daily-burn measurements. See experiment definitions.


`publication*` retains the complete22-group/66-pair/132-row publication matrix,
with three independent pairs each. All overall candidate rows improve; reductions
are2.3849–39.4538%ICP and2.3794–36.1586%generic. Alias additions mean one newly
published binding in the stated2/4,096-binding complete table, with fullvalidation.
No-op Hist verification is preserved. These combined source+compiler results are
separate from the component attribution and canonical production rows.


`small-receivers*` retains all4groups/12pairs/24rows at1/16relations, with
separately installed authenticated full-record self/external/neuron receivers.
All exactpayload/caller/count comparisons pass. Reductions1.8248/5.9426%ICP and
1.8769/5.4003%generic; allthree samples agree. Configuration isdebug, notcanonical
production. Sender/recipient costs remain separate in the larger matrix.


`scheduler*` retains12groups/36pairs/72rows of actual unchanged configured-debug
scheduler windows. Allgroups pass3pairs, with equal polls/callbacks/endinglag.
The24hour Standard rows require144polls,24funding runs,onepricing anddailyaliasrefresh.
Settledwindow+explicitcontrol/drain savings: one realglobal+specific subscriber
0.6356%ICP/0.7917%generic;1024idleglobals11.7071%/8.7804%. These differ from
fixed-work savings. Rawwindow debits remain separate. Continuous660second real
fixtures complete664polls (fourtrafficingress opportunities), idlefixtures660;
bothvariants agree. One explicitround/second plusingress, or600secondsteps with
20rounds/step forday, are latencyassumptions. Debugruntimecosts omit production
principal parsing; no canonical daily-burn or mainnet-runway claim. Fresh models are retained separately.


`component-ab*` isolates the combined immutable-context/borrowing prototype at
originalz/raw, baseline→cdc555b (17groups/51pairs/102rows). Every functional
comparison passes. Generic strictlyincreasing amounts regress2,392,763cycles
(0.0301084%,zero observedrange); allother16rows improve. On that path every
qualifying observation still needs a newmaximum clone, so the borrow-based path
cannot save those clones. Additional indirection/code-generation cost is a
plausible inference, not a proved instruction-by-instruction cause. Retain the
small explicit worst-case trade-off with equal/shared and arbitraryprecision wins;
the finalshrunk/profile2 combined increasing row improves7.05%. Do not add gains
across these separate prototypes or describe allindividualrows as improvements.


`component-ab-admissions*` adds10groups/30pairs/60rows atsamez/raw. Allfunctional
comparisons pass. EarlyA/B new/fullwidthranges regress0.1367/0.1369%ICP and
0.3381/0.0487%generic; duplicate/threshold rows improve0.44–0.47%. Onepayout
cannot amortise scan-context setup, and thisprototype predates relation-key reuse
and the capacity correction; this is a code-path explanation, not exactinstruction
attribution. Finalcombined ranges improve36–39%. Intermediate regressions remain
visible; no separate-percentages are added together.


`component-c*` isolates scan-owned accumulation, AB→C atz/raw (16groups/48pairs).
Everyrow passes and improves: growing-map savings3.3630%ICP/2.5790%generic at
64pages,5.3352%/4.3277%atonepage. Theonepage gain includes avoiding merge-back;
longscans also eliminate repeated full-map copies. Repeated watched16-recipient backlog
savings are0.0466–0.2110%. Failedscan discard/pagecommit proof is separate from
these successful-cost rows. No independent per-page256-cap was introduced.


`component-d.csv`/summary/input/receipts supersede older narrowD-adverse data with
the stronger settlement cohort:27groups/81pairs/162rows, C→D atz/raw. All
functional comparisons pass. Equal256×1024 saves21.5430%ICP/21.4936%generic.
Sevenrows regress0.0100–0.0425%: singleton, increasingICP, uniqueunwatched and
uniquewatched workloads. There is no skipped evaluation on increasing records;
extra comparisons/record cloning and larger cache entries trade against repeated
walk elimination. Exact codegen attribution is unavailable (generic increasing
slightly improves0.0134%). No allocatedlinear-memory increase occurs; oneICP32
transfer group uses onefewer64KiBpage. Extra live summary storage remains atmost
oneownedNat per multi-watcher destination in the bounded page; no liveheap counter.
Native independent oracle and generation invalidation tests establish correctness.


Workload-label correction: `backlog` repeats amount20 to watchedtarget7 with16
watchers on everypage; it is not uninterested traffic. `growing` first processes
512competing identities, then mostly unwatched increasing targetnumbers and a
final update to retainedtarget0. The36.7097%overall ICPbacklog result describes
repeatedwatched traffic. Rawscenarios/fixtures/numbers are unchanged. Separate
`ordinary` workloads are completely unwatched. Use these exactlabels inreports.


`component-e*` isolates lazyglobal snapshots, D→E atz/raw (16groups/48pairs).
Allrows pass and improve. Idle1024globals saves6,254,664cycles/32.1074%ICP and
4,898,218/20.1499%generic; emptyregistry savings are only0.0066–0.0120%. Bootstrap
andarchive-only savings preservezerowakes. Native traversal counters independently
prove zeroglobal visits onempty pages and directmapcount. Eligibility/order tests
covernewadmissions; no cross-pollregistry mirror is added.


`component-f*` isolates global-only encoding reuse, E→F atz/raw (12groups/36pairs).
Allfunctional rows pass. Global1024 saves3.1916%ICP/3.2523%generic;global256
3.1365%/3.1877%;mixed1.5621%/1.6276%. Single-global savings only0.043–0.049%.
Four specific-only controls regress0.0053–0.0057%, about329cycles/recipient from
an additionalglobal-only selection path (exactinstructioncause notisolated).
Retain thissmallbranchcost for eliminated repeated encodings; messagecost/call
count/payload/reservepolicy unchanged. Native typed/raw exactbyte proof is separate.


`component-g*` is the early publication prototype F→G atz/raw (22groups/66pairs),
not the final implementation. Allfunctional checks pass. Newrange saves11.5067%
ICP/1.8727%generic;unchanged4096aliases10.3547%/10.0644%. Eight rows regress:
repeatedself/external2.00–3.46% from repeating derivation/lookup instead of caching
a knownno-op, corrected inI; globaldeclarations0.0046–0.0249%. The latter show no
isolated cyclewin forglobalpublication even though native stable-write counts
fall; they are an explicit small stable-I/O trade-off, not a claimed saving.
WholeG measurements do not isolate everylookup from otherpublicationcodegen.
Final Icorrection and combinedrows must be assessed separately.


`component-i*` measures the final known-noop cache/borrowedindex correction G→I
atz/raw (8groups/24pairs). Functional rows pass. Repeatedself/external declarations
improve2.3771–3.6106%, recovering G's redundant lookup cost. Newranges regress
0.0750%ICP/0.0655%generic, and genericthresholdrow0.0021%, reflecting required
clean/dirty bookkeeping; keep these tinytrade-offs explicit. Native proof counts
256knownnoops→1keyderivation/0writes, without dropping an earlier improvement.
J's separate exact-capacity correction and finalcompiler effects are not added
arithmetically to these percentages.


`component-j-{unique,range}*` retains the earlier6pairs I→J atz/raw: exact ICRC
key-buffercapacity correction. Generic256uniqueunwatchedaccounts saves388,352
cycles(0.3904%);new256range saves424,093(0.3606%). Threeindependent samples each,
no memoryallocation-high-water increase. These are explicitly the00c0d63 older
harness cohort: no sends/timeadvance and immediate=settled in both, so no delayed
301second accounting claim. The newrangeIcost117,599,231 agrees with the stronger
cohort. Do not blend differentcohorts or add percentages; finalJ/2matrix is separate.

`source-gates.json` records the full412-test pass at d003486 and the fresh OSV
failure without a new exception. `production-real-preflight*` contains ten
one-sample fixture comparisons at53ca069; these are not final canonical headlines.
`launch-*-preflight.csv` verifies identical actual launch balances after excluded
setup/upgrade credits, with exact sender balance subtraction. Initial global
fixture failures and their dispositions have digests in `preflight-failure-receipts.json`.

`scheduler-boundary-diagnostic.*` reproduces the ICP day raw-debit increase0.0363%
and isolates the tail. All three lanes are idle; there is no later drain work.
First closing ingress costs55,440,266baseline/6,635,620candidate; the same setter
repeated at the same clock costs6,622,279/6,593,314. The cold/boundary difference
is consistent with periodic memory billing, not proved to an exact replica step.
Preserve both raw and settled-plus-control metrics; neither establishes a mainnet
daily saving. The simple setter changes only the existing debug override Cell.
