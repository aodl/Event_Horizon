# Cycle optimisation validation

Release acceptance is **blocked** by the unsuppressed OSV metadata finding and
required PR/CI evidence. The branch is now published; see the
[release-closure follow-up](cycle-optimisation-release-closure.md). The implementation is complete; the local
validation outcomes below distinguish completed checks from those blockers.

## Source and artifacts

Baseline: `81c5a0d72ff188f5ee9f412deb20566a12307642`, independently preserved and
built in a clean worktree. Its reviewed backend/frontend hashes were reproduced:
`2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528` and
`2658a8abc721c4f92f4af2c1ce3116bad08826c1b21aca6e18327515314754c3`.

Immutable validated source: `7cfa8b354fa9621786a09b13c9a6dff498872b19`. Runtime
changes conclude at `00c0d63`, profile selection at `9e304e1`; later source changes
are test support, evidence and the frontend backend pin. This report's evidence
commit follows the validated revision and changes only docs/data/manifest. The
[reproduction guide](cycle-optimisation-experiments.md#evidence-only-final-commit)
provides the diff and artifact checks without embedding this report's own hash.

| Production artifact | SHA-256 |
|---|---|
| Backend | `f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970` |
| Frontend | `46d7b10673b7c37dce7ec773a0829bf104723b57b7fe2370d5838bc8df3b4a24` |

The supported `npm run verify:reproducible-artifacts` passed two independent
uncached Docker builds; `./tools/scripts/docker-build` then populated outputs.
All four files (both Wasms, artifact manifest and build metadata) are byte-identical
A = B = populated. Export/Candid audits passed. The
[canonical receipt](cycle-optimisation-data/canonical-validation.json) binds exact
source, tools, hashes, commands and raw-log digests.

The frontend's registry, locked bundle and associated check pin the actual backend
hash above. Backend size is 1,847,228 bytes; frontend 4,009,935. The changed backend
hash follows the implemented optimisations and selected compiler profile.

Production changes are confined to polling, state publication, account derivation
helpers, ICRC-3 extraction, notification sending and daily-health counting. Other
changes are the release profile, frontend pin, test-only dependency edge, fixtures,
benchmark tools, evidence and manifest. The
[complete changed-path list](cycle-optimisation-data/changed-files.txt) includes
source, fixtures and evidence. No package version changed: Tokio remains
1.51.6 in native PocketIC tooling and absent from both production Wasm graphs.

## Dispositions and invariants

| Candidate | Disposition |
|---|---|
| A | Implemented immutable scan context and reusable relation keys. Mutable alias, price, evidence, fee, balance and health reads retain their logical positions. |
| B | Implemented borrowed Nat/memo extraction and matching; clone only retained insertion/new maxima. Sender/memo validation and full-width conversions remain. Exact ICRC key capacity fixes allocation without changing bytes. |
| C | Implemented one scan-owned accumulator, returned only on successful completion. Failed scans cannot dispatch it. Page admission/cursor transactions and previously committed progress remain independent. |
| D | Implemented page-local record highs per effective watcher generation. Added/easier relations invalidate the whole cache entry. Empty/singleton lists avoid a summary allocation. |
| E | Implemented lazy eligible global snapshots and direct stable-map count. Shared snapshot precedes the first live block/admission; generic snapshot follows successful observation and precedes admission. |
| F | Implemented one ordinary Candid encoding per global-only batch. Each direct call retains its immediate actual-cost reserve check, oneway semantics, ordering and failure continuation. |
| G | Implemented unchanged-write suppression for relations/globals/aliases. Known no-ops retain effective values with dirty=false; later duplicates cannot discard staged improvements. Complete alias validation/timestamps remain. |

Record-high correctness depends on chronological processing: a later amount no
greater than a processed high cannot newly qualify an unchanged threshold, lower
its first index, increase its maximum or open retention capacity. Invalidation
starts a fresh generation after the admitting block. The single accumulator keeps
the same first 256 identities and permits later updates after overflow. Only the
guarded poll writes production memberships, making deferred global snapshots safe.
No stable-map/RefCell borrow crosses an await. Deterministic ordering means sender
attempt order; remote execution order is not guaranteed.

Historian call suppression is deferred because exact-route evidence, mutable
alias/price timing and failure observations are contractual. Threshold sorting is
deferred: qualifying increasing values still need every maximum updated, while a
new index complicates retention/admission order. The separate unmatched-increasing
production controls retain their full walks and zero callbacks; a sorted index
could help those walks, but no additional saving is claimed without an independent
index design/proof. Combined measured costs already improve on that path.
Cross-page/global heap mirrors are deferred: repeated one-destination 64-page traffic could avoid 63 lookups, but
unique traffic offers little reuse and lazy empty polls already enumerate zero
globals. No cycle gain is inferred from that operation bound. Stable replacement
and financial batching are not authorised.

## Measurements and trade-offs

| Workload | ICP sender cycles: baseline → candidate (reduction) | Generic sender cycles: baseline → candidate (reduction) |
|---|---:|---:|
| Ordinary 256 blocks | 119,843,076 → 89,725,994 (25.130%) | 124,305,728 → 112,025,529 (9.879%) |
| Idle, 1,024 globals | 52,838,100 → 46,226,748 (12.512%) | 57,659,859 → 52,259,975 (9.365%) |
| New 256-target range | 191,064,359 → 104,998,580 (45.045%) | 191,877,941 → 116,991,204 (39.028%) |
| 256 global receivers | 1,442,204,173 → 1,394,388,057 (3.315%) | 1,448,866,740 → 1,400,250,401 (3.355%) |
| 256 equal transfers, 32 receivers | 319,260,898 → 245,136,207 (23.218%) | 323,748,013 → 267,336,855 (17.424%) |
| 256 increasing transfers, 32 receivers | 319,568,192 → 275,600,316 (13.759%) | 323,981,073 → 297,938,381 (8.038%) |
| Increasing, 32 unmatched relations | 103,523,773 → 69,629,041 (32.741%) | 107,724,733 → 91,832,074 (14.753%) |
| 4,096 blocks, 512 relations/overflow | 1,698,876,735 → 1,008,879,629 (40.615%) | 1,836,724,686 → 1,443,640,227 (21.401%) |

All 24 canonical groups pass and improve; every group has three independent
samples. The raw CSV and medians/ranges are tracked in the data directory.

These are genuine uninstrumented production modules against fixed-principal mocks,
three independent serial samples per case and ordinary execution limits. The old
seven-case cohort uses 6T/Standard/600 seconds; real-receiver cases use
80T/VeryFast/10 seconds. Actual launch balances are equal after excluded fixture
credits, which are recorded separately. Every intended real receiver authenticates
and decodes its complete notification; incomplete scans or missing sends reject
results. Dependency and recipient debits remain separate and include their stated
storage interval. Request/instruction attribution unavailable in canonical modules
is supplied by identical test-only shadows, not a production profiling API.

Configured-debug data remains separately labelled: equal 256×1,024 improves
25.344% ICP/25.268% generic; increasing improves 7.067%/7.050%; repeated watched
64-page ICP traffic improves 36.710%. All 93 main groups and 22 publication groups
improve overall. Generic 32,769-bit stress is separate from bounded ICP amounts.
The historical shared-account immediate baselines reproduce 5.958/7.834B; settled
rows add the declared 301-second interval. Historical global fan-out used the
separate-ledger architecture, so its approximately 5.742B is not an ICP-only row.

Individual source effects, adverse rows and raw receipts are in the
[data index](cycle-optimisation-data/README.md). A/B's generic increasing prototype
regresses 0.0301%; D's worst isolated regression is 0.0425% on increasing ICP;
F's specific-only path adds about 329 cycles/recipient. Early G no-op regressions
of 2.00–3.46% are corrected by the known-no-op cache, which adds at most 0.0751%
on new ranges. Global insertion suppression shows no isolated cycle win and is
retained as a small, rare-declaration stable-write trade-off. Percentages are not
added across prototypes.

Synchronous source-only production probes reduce ordinary runtime construction
258→2 and account hashes 256→0; range construction 515→2 and hashes 1,026→258.
Equal 256×1,024 matching walks 256→1, not a 256-fold total-cycle saving. Accumulator
copies fall 64→0. Counters never span awaits; nested spans overlap. Shadow effects
are quantified separately and never substituted for uninstrumented costs.
The eight final matched-host production shadow diagnostics differ from their
uninstrumented medians by -0.0128% to +2.3637%; these one-sample diagnostics
are retained separately, without applying a correction factor to headline costs.

Measured main-suite stable allocation remains 100,728,832 bytes and maximum
allocated linear memory 3,735,552 in both variants; no row grows allocated heap.
This is not a live-object profiler. Context keys add bounded per-page pending
storage; record-high summaries add one owned Nat per multi-watcher destination.
No cross-poll cache or stable region is added.

The final profile is workspace opt-level 2 with unchanged Rust 1.94.1, LTO,
codegen-units=1, panic/overflow/target settings and pinned ic-wasm 0.9.7 shrink.
All 48 z/s/2/3 screening pairs pass; 3 has marginal/mixed gains and larger files.
Against refactored z, 2 increases backend/frontend bytes 29.03%/6.71%. Frontend
screening installation cost rises 11.7%, reported idle cost 0.55%, allocated heap
unchanged; asset bytes match. Query statistics counted 13 of 20 known calls,
so no exact query-instruction claim is made.

## Fixed time, equivalence and gates

Actual configured-debug scheduler fixtures keep the existing policy: 144 polls,
24 funding runs, one pricing run and daily alias refresh over a Standard day;
equal callbacks and zero ending lag. Continuous 660-second fixtures complete
664 polls with four traffic ingress opportunities, 660 without, in both variants.
One round/second or 600-second steps/20 rounds are explicit latency assumptions;
these are not canonical production daily-burn measurements.

The raw ICP one-receiver day debit rises 0.0363%; post-closure accounting falls
0.6356%. All lanes were idle; first closing ingress costs 55.44M/6.64M, while the
same setter repeated at the same clock costs 6.62M/6.59M, with no later drain work.
This is consistent with periodic billing/cold state, not traced to an exact replica
operation. Both metrics are retained without claiming unconditional daily savings.
Larger code raises reported idle billing by about 11.64M/day in that fixture.
Fresh idle-only model projections omit unavailable maintenance/storage costs;
explicit 1/2/5/10-second completion assumptions show longer Continuous residence,
not a mainnet runway guarantee. See [model evidence](cycle-optimisation-data/models/README.md).

Independent 32×512 randomized traces, complete logical stable IDs 0–11 at three
checkpoints, controlled alias/price interleavings, threshold reductions and
later-page overflow updates pass. No financial field is normalized away. The
Historian delay test uses an explicit test-only update wrapper because replicated
queries cannot await; normal production fixtures retain the query endpoint.
Reviewed 81 baseline upgrades preserve relations, aliases, cursors and pending
notify/transfer states, then exercise/upgrade again with exactly one surplus
transfer. Historical 44d83c0 coverage remains.

| Gate | Result |
|---|---|
| fmt / strict workspace, all-targets Clippy | Pass; repeated with all features at validated source |
| test_unit | Pass: 146 Rust + 138 frontend |
| test_all | Pass: 128 PocketIC; total 412, including maximum callback and zero-argument Index regressions |
| Independent state/interleaving/upgrade comparisons | Pass; complete stable IDs 0–11 and historical 44d coverage retained |
| Final canonical production | Pass: 72 uninstrumented pairs, eight shadow diagnostics, two baseline-production upgrade scenarios |
| Final canonical frontend | Pass: 20 certified HTTP response checks; backend pin equals actual backend hash |
| Manifest/static/Candid/production exports | Pass; no debug or benchmark export in production |
| Two-build reproducibility / population | Pass: all four outputs equal from frozen committed source |
| Fresh exact-policy security | Audit/deny/npm pass; OSV blocked as below |
| Candidate GitHub security CI | Unverified; source cannot be published with available credentials/integration permissions |

Four PocketIC HTTP `IncompleteMessage` errors stopped fan-out shards;
none produced an accepted row. Reruns preserved the service inputs and passed. Failed and successful
log digests are retained separately; no production retry or assertion changed.
The remaining native test processes use `MALLOC_ARENA_MAX=2` on both sides, a
[glibc host allocator bound](https://sourceware.org/glibc/manual/latest/html_node/Memory-Allocation-Tunables.html)
that changes no Wasm allocator, IC metering, clocks or call budgets. All raw fields
except sample match the earlier default-host global sample exactly. The transport
cause remains unconfirmed; this is an environment mitigation, not a source win.

Two rejected preflights are preserved: 256 sequential seed admissions exceeded
PocketIC's 100-round ingress wait; 6T then reserve-skipped 140/256 sends because a
65-byte call needs 42.102B available including reservations. Sixteen-item seed
batches and equal 80T actual launch liquidity resolve those fixture assumptions.
No production limit, reservation, assertion, retry or service rule was weakened.

The fresh exact-policy scan passes cargo-audit 0.22.1, cargo-deny and npm audit,
but OSV 2.3.8 fails on GHSA-qwgh-2vcv-g2f7/block-buffer 0.10.4. The maintainer's
affected range begins 0.11.0; the locked implementation uses a separate cursor.
This supports a metadata discrepancy, not a security-gate waiver; see the
[investigation](block-buffer-osv-investigation.md). No new exclusion or dependency
refresh was introduced. In the original session, Git push lacked HTTPS credentials and connected branch
creation returned 403. The branch has since been published at 9528da7; this
historical failure does not describe its current visibility. PR/CI closure is
recorded in the follow-up. No candidate CI success is claimed here. After publication,
verify that exact commit in [Dependency Security](https://github.com/aodl/Event_Horizon/actions/workflows/dependency-security.yml).

APIs, stable IDs/encodings, memo/evidence rules, notification/Index contract,
page commits, financial safeguards and cadence policy are unchanged. No mainnet
or real-funds action, reinstall, controller change or other-project write occurred.
