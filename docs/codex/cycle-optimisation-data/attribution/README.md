# Synchronous attribution

All 56 independent instrumented pairs passed their functional comparisons. Each
row is one deterministic diagnostic sample, not a three-sample cycle headline.
`inputs.json` binds the six shadow modules, mocks, host harness and instruments;
`receipts.json` retains CSV/metric/log digests. Reproduce with the tracked
`tools/instrument-cycle-optimisation.py` and
`tools/run-cycle-optimisation-attribution.py` commands described in the experiment
guide. Instrumentation never enters the canonical release artifacts.

`metrics.csv` contains actual operations and synchronous instruction spans.
`candidate-z` isolates source changes; `candidate-2` includes the selected profile.
Counters use the pinned CDK's `instruction_counter()` (message counter zero)
within one synchronous message segment.
Admission preparation stops before its Historian await; the ICRC page body has no
await. Request spans count call construction, not asynchronous execution totals.
Nested spans overlap and must not be added together.

For the production ICP 256-block no-memo scan, source changes reduce runtime
configuration construction 258→2 and legacy account hashes 256→0. Runtime-span
instructions fall 19,969,033→153,120. A 256-target range reduces runtime creation
515→2 and hashes 1,026→258 while retaining one exact-route Historian call and 256
publication operations. Inclusive staging instructions fall 50,852,234→20,949,959;
publication 89,796,296→63,692,987. These inclusive numbers are not additive.

Configured-debug equal 256×1,024 matching walks 256→1 and matching-span
instructions fall 1,832,473,411→9,593,052 at the original profile. The 64-page
growing fixture retains 64 Ledger requests but removes all 64 whole-accumulator
copies; the baseline copy spans alone consume 13,655,295 instructions. This
excludes merge-back work and does not claim a 256-fold total-cycle improvement.

`shadow-overhead.csv` compares selected-profile shadows to matching uninstrumented
three-sample medians. Observed debug/publication differences range −0.2534% to
+2.1888%: probes, altered code generation and storage footprint can change cost in
either direction. No universal overhead correction is subtracted from headlines.
Production overhead is evaluated separately against final canonical measurements.
`linear_heap_pages` is allocated linear memory, not live allocations or a complete
peak-heap profiler.
