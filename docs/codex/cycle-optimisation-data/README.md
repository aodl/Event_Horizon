# Cycle optimisation data

Compiler screening source: runtime00c0d63; host harness8802517. Inputs/digests
and raw pre/post-shrink sizes are retained here. These48complete independent
paired scenarios use **one screening sample per row**. They are not the final
three-sample headline matrix, canonical release hashes or daily-burn evidence.
The frontend still carries the old reviewed backend pin during compiler-only
screening. Final pin and canonical artifacts are separate release gates.

Profile2 is selected provisionally over3: increasing/unique workloads slightly
favour2;3 has small wins elsewhere, while adding1.27%backend/1.54%frontend
size over2. Against z, profile2 adds29.03%backend/6.71%frontendbytes.
All production export audits and48paired behaviour assertions passed. Full
behaviour/performance/reproducibility acceptance against2 is still required.

Frontend queries verified20complete/ranged certified responses against source
assetbytes perprofile. Status query statistics contain aggregation information
loss (13reported calls for20known requests); raw estimates are not exact
per-request instruction counts. Installation costs are single-sample screening
values. See ../cycle-optimisation-experiments.md for commands and limitations.


The final-v2 uninstrumented fixed-work matrix is complete: 93 groups × three
independent pairs (279 pairs/558 raw rows). `fixed-work.csv` retains all fields,
`fixed-work-summary.csv` gives medians/ranges for each group, and the input/receipt
JSON binds artifacts, host harness, mocks, lock/tool versions and raw-log digests.
All overall candidate rows improve; ICP reductions range1.6429–36.7097%, generic
1.2871–34.8588%. Zero observed ranges reflect deterministic PocketIC fixtures,
not mainnet uncertainty. Stable allocation is100,728,832bytes for both; largest
allocated Wasm linear memory is3,735,552bytes in either variant. Per-change
regressions and publication/scheduler/canonical data are separate and stillpending.
These are configured-debug fixed-work costs with declared settlement intervals,
not canonical production or daily-burn measurements. See experiment definitions.


`publication*` retains the complete22-group/66-pair/132-row publication matrix,
with three independent pairs each. All overall candidate rows improve; reductions
are2.3849–39.4538%ICP and2.3794–36.1586%generic. Alias additions mean one newly
published binding in the stated2/4,096-binding complete table, with fullvalidation.
No-op Hist verification is preserved. These combined source+compiler results are
separate from the pending per-change attribution and canonical production rows.


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
principal parsing; no canonicaldailyburn/mainnetrunway claim. Models remainpending.


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
