# Cycle optimisation validation

Release acceptance is **pending**. Runtime source/profile are fixed; complete
canonical pairing, final production measurements, security and CI outcomes are
recorded below as they become available. No deployment is authorised or performed.

## Sources and recipe

Independent baseline: `81c5a0d72ff188f5ee9f412deb20566a12307642`, preserved in a
clean detached worktree and independently built. Its canonical backend is
`2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528`, frontend
`2658a8abc721c4f92f4af2c1ce3116bad08826c1b21aca6e18327515314754c3`.
Checked-in debug/production companions have explicit SHA-256 provenance.

Runtime changes conclude at `00c0d63840508414b39b91535631298ee75759ee`; profile
selection at `9e304e1`. Later commits add tests/evidence only until the frontend
pin pairing. The frozen measurement host is `8c65325`; complete source gates run
at `d003486781551b35ce8985b44622297bd8fa343e`. Final pairing SHA and frontend hash
remain pending. The initial canonical build at afec92cad49dfa7df4d6d2a795d50a91447c46a4
produced backend `f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`,
matching the independent selected-profile build. Its changed hash follows the
implemented backend optimisations and compiler selection. The frontend pin and
locked bundle now use that actual hash; final two-build pairing remains pending.
This report identifies immutable validated revisions rather
than including its own commit hash.

Selected release recipe: workspace `opt-level=2`, Rust1.94.1, unchanged LTO,
codegen-units1, panic/overflow/target settings and pinned ic-wasm0.9.7 `shrink`.
The dependency graph retains Tokio1.51.6 and all package versions. The sole lock
change adds an existing stable-structures crate as a native test dependency;
neither production Wasm graph gains a dependency. No security policy changed.

Runtime files changed: polling/state/subscription, ICRC-3 extraction, callback
sender and daily-health count. Config/account algorithms, financial state
machines, cadence, Candid definitions and stable schemas are unchanged. Additional
files are host fixtures/oracles, benchmark tools, evidence and the source manifest.

## Candidate dispositions

| Candidate | Implemented change; evidence and trade-off |
|---|---|
| A | Scan-scoped immutable trust anchors/identity/default accounts and reusable relation keys. Production ordinary256 runtime reconstructions258→2; account hashes256→0. Memo-free early rejection preserves validation and side effects. Single-range early prototype costs +0.049–0.338% before later key/publication improvements. |
| B | Borrow Nat/memo fields and match amounts; clone only retained insertion/new maximum. Validate sender and memo types even when unused. Borrow u64 conversion and reserve exact ICRC key capacity without changing bytes. Early generic increasing prototype +0.0301%; each observation still requires a new maximum. |
| C | One scan-owned accumulator; only successful CompletedScan can dispatch. Whole-map copies64→0 on64pages. Source-only growing64 saves3.363%ICP/2.579%generic; page admissions/cursors remain transactions. |
| D | Page-local record high tied to effective watcher generation; no summary for empty/singleton lists. Equal256×1024 matching walks256→1, source-only cycles −21.543%ICP/−21.494%generic. Worst isolated regression +0.0425% on increasingICP; unique/singleton overhead ≤0.0381%. |
| E | Lazy eligible global snapshot and direct stable-map count. Empty1024globals source-only −32.107%ICP/−20.150%generic; emptyregistry effect0.007–0.012%. No empty/archive wake or admission-eligibility change. |
| F | Ordinary one-record Candid encoding reused for global-only recipients; individual direct calls and immediate actual-cost reserve checks remain. Source-only1024global −3.192%ICP/−3.252%generic; specific-only controls +0.0053–0.0057% (about329cycles/recipient). |
| G | Suppress unchanged relation/global/alias writes; preserve staged improvements and complete alias validation/timestamps. Known-no-op cache correction recovers an early2.00–3.46% regression. Correction adds ≤0.0751% on new ranges. Global insertion alone shows no isolated cycle win (+0.0046–0.0249%); retain its bounded stable-write trade-off explicitly. |

All component tables retain adverse rows and exact deltas in the
[data index](cycle-optimisation-data/README.md); percentages are never added.
The ICRC capacity correction separately saves0.3904% on unique unwatched accounts
and0.3606% on a new range in its explicitly labelled older cohort.

The record-high proof depends on chronological processing: amount≤a prior high
cannot newly qualify an unchanged threshold, lower a retained first index,
increase its maximum or open capacity. Added/easier relations invalidate the
entire destination cache entry, starting a fresh generation after the admitting
block. A new page resets summaries. One accumulator preserves the exact first256
distinct(subject,target) identities and later updates after overflow.

SharedICP snapshots globals before the first live block/admission. Generic
observation snapshots only after successful activity, before its independent
admission scan; the guarded poll is the only production membership writer.
Aliases, prices, evidence, fees, balances and health retain existing logical read
points. No stable/RefCell borrow crosses an await. Failed scans drop transient
notifications, preserving earlier committed pages and explicit archive progress;
generic admission failure cannot suppress successful observed notifications.

| Higher-complexity investigation | Disposition |
|---|---|
| Covered declarations/Historian | Deferred: exact-route requests, mutable alias/price timing, failure and recovery observations are contractual. Every no-op declaration still verifies its exact memo route. No cached totals, pooling, batching or call suppression. |
| Threshold-sorted index | Deferred: increasing qualifying amounts must still update every retained maximum; a threshold index would also need exact within-block retention order and admission invalidation. The simpler shortcut has measured gains and bounded adverse rows. Unmatched increasing production coverage is separate. |
| Cross-page watcher cache | Deferred: repeated one-destination64pages could avoid63of64stable lookups; mostly unique growing destinations offer little reuse. No cycle saving is inferred from that operation bound. A poll-wide destination cache would lose the current bounded-page memory property and complicate admission recovery. |
| Global heap mirror | Deferred: lazy empty polls already enumerate zero globals. Active global1024 still requires recipients; avoiding registry reads would introduce all-subscriber heap state and invalidation/upgrade paths. |
| Stable replacement/financial batching | Not authorised; IDs0–11, encodings and durable money-moving boundaries remain unchanged. |

## Fixed-work and attribution results

These are **uninstrumented configured-debug** medians of three independent serial
pairs, not canonical production costs. All three samples agree in each group;
zero observed range does not imply zero real-world uncertainty. Ordinary execution
limits, fixed fixtures/clock/balances and identical frozen mocks are used. Sender
debits, recipient/dependency debits, payload bytes, requests and cursor progress
are separate. Sending scenarios retain declared301second storage/settlement time.

| ICP workload | Baseline cycles | Candidate cycles | Reduction |
|---|---:|---:|---:|
| Ordinary256, no interested accounts | 66,885,589 | 56,319,701 | 15.797% |
| Equal256,1024watchers | 7,846,795,403 | 5,858,110,163 | 25.344% |
| Increasing256,1024watchers | 7,922,889,782 | 7,362,979,938 | 7.067% |
| Repeated watched64pages,16watchers | 5,403,239,535 | 3,419,727,870 | 36.710% |
| Growing/overflow64pages, final retained update | 5,164,296,876 | 3,801,091,383 | 26.397% |
| Global1024 | 5,744,359,860 | 5,555,701,382 | 3.284% |
| Idle1024globals | 19,489,546 | 12,873,139 | 33.948% |
| New256-target range | 117,032,509 | 70,858,713 | 39.454% |
| Unchanged4096aliases | 827,871,139 | 524,753,817 | 36.614% |

All93main groups improve overall. Generic equal256×1024 improves25.268%,
increasing7.050%, and32769-bit stress32×16 improves34.859%; bounded ICP amounts
are reported separately. Four small real-receiver groups improve1.825–5.943%ICP
and1.877–5.400%generic with full authentication/decoding. Twenty-two publication
groups improve; alias additions mean one new binding in a complete2/4096table.

Measured stable allocation is100,728,832bytes in both variants; maximum allocated
linear memory3,735,552bytes in both. No paired main row grows allocated heap;
21pairs use one64KiBpage less. This is not a live-object/peak-heap profiler.
Record-high summaries add one owned Nat per cached multi-watcher destination in
a bounded page; unique/increasing comparison/allocation overhead is disclosed.

The56instrumented diagnostic pairs use identical synchronous scopes, pinned
message instruction counters and no subtraction across awaits. Source-only
ordinary production runtime-span instructions19,969,033→153,120; range staging
50,852,234→20,949,959; publication89,796,296→63,692,987. Nested spans overlap and
are not added. Shadow effects range−0.2534%to+2.1888% on matched debug/publication
rows; headline cycles come from uninstrumented builds. See
[attribution](cycle-optimisation-data/attribution/README.md).

## Compiler and scheduler

All48predeclared z/s/2/3screening pairs pass after the pinned shrink transform.
Profile2 beats z in intended workloads;3has marginal/mixed gains and larger files.
Screened backend bytes z1,431,613→2=1,847,228 (+29.03%); frontend3,757,901→4,009,935
(+6.71%). Frontend installation rises11.7%, reported idle cost0.55%, allocated
heap unchanged. All20certified responses/profile match source assets. Query
statistics reported13for20known requests; no exact query-instruction claim is
made. Final canonical sizes/hashes are recorded separately after pinning.

Actual unchanged-scheduler configured-debug runs use one round/second over660s,
or600second steps with20rounds for24h. They omit production principal parsing;
these are not canonical/mainnet daily-burn measurements. Both variants complete
the same polls/callbacks with zero ending lag. Continuous660s completes664polls
with four traffic ingress opportunities,660without; no physical throughput gain
can be inferred at that fixture granularity. Full-day Standard completes144polls,
24funding runs, onepricing run and dailyaliasrefresh in both variants.

Settled day plus explicit control/drain improves0.6356%ICP/0.7917%generic with
one real global+specific receiver,11.7071%/8.7804%with1024idleglobals. The raw ICP
one-receiver window instead rises0.0363%; its outstanding-tail investigation is
pending. Larger code increases the reported storage idle rate in that fixture by
11,639,228cycles/day. Raw debit, settled consumption and storage rate are retained
separately; fixed-work savings are not daily savings.

Fresh existing-model projections cover idle ordinary polling only: no invented
maintenance/storage coefficients, no traffic/incoming funding, current policy,
explicit1/2/5/10sContinuous completion assumptions. At110T/1s, zero-global first
downgrade moves3,780,616→3,883,961seconds,1024global2,565,478→3,884,057. Cheaper
polls can extend faster service. See[model inputs/results](cycle-optimisation-data/models/README.md);
no total-system runway or mainnet guarantee follows.

## Behavioural and release gates

Focused equivalence passes: independent linear32×512random traces; exact complete
stable logical contents ofIDs0–11 at three checkpoints on both architectures;
controlled alias publication and a price epoch between Historian awaits; threshold
reduction and later-page overflow updates. No financial field is normalized away:
the fixture controls the supplied balance to exercise the same sampled minimum.
Physical B-tree allocation history may differ after avoided writes.

Verified81baseline→candidate upgrades pass both architectures with representative
pending notify/transfer states, aliases, cursors, self/external/global relations
and lowered thresholds, then exercise/upgrade again with exactly one surplus
transfer. Genuine baseline production modules are used as well as debug state
fixtures. The historical44d83c0regression remains. Final canonical reruns remain
pending. Actual-cost reserve±1/equal probes pass cold/warm global and large payload
cases. A baseline wrapper exceeded40Bwhile doing three encodings plus decode;
this is not a production limit, and full256×32769receiver coverage is retained.

| Gate | Current evidence |
|---|---|
| fmt; strict workspace/all-targets Clippy | Pass at d003486 |
| test_unit | Pass146Rust+138frontend=284 |
| test_all / maximum callback / Index | Running; exact final totals pending |
| Manifest/static/Candid/export | Source checks pass; final canonical binary audits pending |
| Fresh security | Final run pending; prior audit/deny/npm pass, OSV failed below |
| Canonical source pairing/backend pin | Actual canonical backend obtained; pin/bundle/static checks and138frontend tests pass; final pairing pending |
| Two uncached independent builds/population | Pending |
| Final-commit GitHub security CI | Unverified: HTTPS credentials unavailable; connected GitHub branch creation returns403 |

Source-location assertions that forced eager global traversal were replaced by
installed-Wasm first/last-block eligibility tests. Typed/raw sender syntax checks
are supported by exact one-record byte/decode tests, standard/Index receiver
regressions and retained actual-cost/oneway/export checks.

Publication attempts did not create a remote review branch: noninteractive Git
push fails for missing HTTPS credentials; connected GitHub create-branch returns
`403 Resource not accessible by integration`. Repository metadata advertising push
permission does not grant this integration Git-reference writes. No final-commit
workflow was triggered or passed. After access is supplied, publish the local
review branch and inspect [Dependency Security](https://github.com/aodl/Event_Horizon/actions/workflows/dependency-security.yml)
for that exact commit. The historical baseline success is not candidate evidence.

Fresh OSV reports GHSA-qwgh-2vcv-g2f7 for block-buffer0.10.4, while the maintainer
affected range starts0.11.0 and the locked implementation uses a separate cursor.
The discrepancy is investigated in[the security report](block-buffer-osv-investigation.md).
No exclusion, allowlist entry or broad dependency update was introduced. Overall
security acceptance remains blocked unless the actual final scan/CI succeeds or
the authoritative metadata is resolved.

All public interfaces, stable IDs/encodings, exact memo/endowment evidence,
notification/Index contract, admission/cursor/error ordering, cadence and financial
policy are preserved. No deployment, reinstall, controller change, funds movement
or write to another project was performed. Remaining release gates above are
outstanding; this report does not claim completed release acceptance.
