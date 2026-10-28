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
