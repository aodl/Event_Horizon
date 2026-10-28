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
