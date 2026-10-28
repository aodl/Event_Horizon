# Cycle optimisation — resumable progress

Baseline: `81c5a0d72ff188f5ee9f412deb20566a12307642`; clean tree, no existing build/benchmark processes at start (2026-10-27).
Independent detached worktree: `/home/codexdev/src/Event_Horizon-baseline-81c5a0d`.
Evidence root: `observations/cycle-optimisation/` (ignored raw artifacts/logs).

## Status — current checkpoint

- Runtime J/profile 2 is fixed (`9e304e1`); harness checkpoint is `8c65325` (reproduction scripts retained in `fef1972`). No production source changed after profile selection.
- Independent `81c5a0d` canonical/debug artifacts remain immutable. A–G implemented; 93 backend units/oracle, strict Clippy, focused failure/invalidation/alias/price checks and reviewed debug plus genuine production baseline upgrades passed. Full release gates remain pending.
- All 48 single-sample compiler pairs passed; profile 2 selected. Actual-cost reserve boundaries passed for global cold/warm, 1×257, 256×4097 and 64×32769 bits. Rejected larger wrapper evidence is preserved; no production limit changed.
- **New measurement queue active: exec7772**, host PID/phase/command in `observations/cycle-optimisation/active-process-v2.json`. Script `run-final-v2.py` serially runs publication/small/scheduler preflights, all final-v2 matrices, then prepared per-component and attribution scripts. No concurrent builds or benchmarks. The old main matrix is stopped; its 261 accepted pair shards under the older settlement check are provisional, not a complete final matrix. Generic growing16 failed a repeatable delayed debit near round 50. An unchanged diagnostic passed 200 drain + 20 quiescent rounds. New final matrices must use a fresh `final-v2/` receipt; do not resume the old `final/` inputs or mix costs.
- Complete logical stable-state differential **passed both ICP and generic** (`complete-state-controlled.log`, 2 tests/72.31 s). All IDs 0–11 compared at three checkpoints, including financial journal. Native SDK dev-dependency adds one lockfile edge only; versions and production graphs unchanged. Initial region 8 mismatch was unequal real liquidity from the debug setter; unchanged financial observe now receives identical 150T, below both actual seed balances. No financial field is normalised away.
- Genuine production `81c5a0d` → local selected-profile-2 candidate upgrades **both passed** (2 tests/85.73 s), with strengthened 2M−1/equal2M threshold boundary. Log `production-upgrade-profile2-exact-threshold.log`. Rerun against final canonical bytes later.
- Final component/attribution/canonical-production measurements, full gates, backend pin/frontend, two-build evidence and final CI are pending. Settlement/state support is committed as `8c65325`; fmt, strict all-feature Clippy, static and manifest checks passed.
- Security gate FAILED on fresh OSV block-buffer 0.10.4 downstream metadata mismatch. No exception or dependency refresh. Final fresh scan and remote CI remain required; no release-completion claim or deployment.

## Resume

Inspect `git status --short`, `git log -5 --oneline`, active process and newest checkpoint. Read exec7772/active-process-v2.json and `logs/final-v2-queue.log`. Growing64 preflight already passed all six ICP/generic pairs with200+20settlement. Queue now runs no-op-global publication, small and short scheduler preflights, then full three-sample debug → publication → small → scheduler into `observations/cycle-optimisation/final-v2/`, followed by component/attribution scripts. Baseline `profiles-baseline/z/event_horizon_debug.wasm`, candidate `profiles-candidate/2/event_horizon_debug.wasm`, fixed normal-z mocks and official PocketIC15. No concurrent builds/benchmarks. Script receipts bind harness/artifact/mock/lock inputs and reject changed resumption.

Afterward run `observations/cycle-optimisation/run-components.py` and `run-attribution.py` serially, summarise complete rows and recalibrate the existing cadence model. Finish full gates and actual canonical backend/pin/frontend/two-build/CI evidence. Draft final report under `observations/cycle-optimisation/drafts/` remains incomplete. No production edit is planned. Read failures and limitations below before accepting any evidence.

## Contract checkpoints

Keep IDs/encodings 0–11, public interfaces, exact routes, mutable alias/price read points, page commits, scheduler/financial safeguards unchanged. Only a successful scan may yield transient matches. Record-high summaries are page-local and invalidated with watcher overlays. No financial journal write suppression.

## Initial harness commit

Serial paired installed-debug-Wasm harness: `tests::cycle_optimisation::paired_fixed_work`. Three independent samples per row; default smoke versus opt-in full ICP/generic matrix. Costs exclude fixture setup and observation queries; 30 settlement ticks at unchanged simulated time. No instruction or peak-heap claim yet. Canonical fixed-principal production harness remains required.

Exact next: `EH_OPT_CANDIDATE=$PWD/tests/fixtures/event_horizon_81c5a0d_debug.wasm EH_OPT_OUTPUT=$PWD/observations/cycle-optimisation/baseline/smoke.csv POCKET_IC_MUTE_SERVER=1 DFX_IDENTITY=codex_local cargo test --locked -p event-horizon-pocketic --lib tests::cycle_optimisation::paired_fixed_work -- --exact --ignored --nocapture --test-threads=1`. Wait for all builds before measuring.

## Active implementation checkpoint (A/B)

Current committed scaffold: `5acaf93`; working tree contains A/B implementation and harness Time-API correction/formatting. Initial harness compile failed due to PocketIC 15 `Time` API; fixed using `.into()` and `as_nanos_since_unix_epoch`. Baseline self-comparison is active (exec 30437, `baseline-smoke.log`); warm fixture builds finish before measured regions. An unrelated B2Z build/benchmark began concurrently: treat this run as **diagnostic only**, not headline performance evidence. No external process was stopped and no external repo written.

A: scan-local runtime and lazily derived Faucet/EH default account identifiers; memo absence rejects before pure address work. Alias, price, governance validation, Historian exact-route calls stay at their old logical points. Stage/publication derivation reuse is still pending.
B: match recording borrows Nat, cloning only insertion/new maximum; ICRC decoding borrows validated Nat/memo. New pointer-identity/large-memo unit test plus existing malformed sender/memo tests protect extraction. `ab-unit.log` (exec 60177) may predate the latest two new unit tests: rerun it before committing A/B.

Next commands: `cargo test --locked -p event-horizon --lib`; verify baseline self-comparison result; build A/B debug Wasm once, then run paired harness with that immutable file, serially in a quiet window. Do not claim concurrent diagnostic costs as final evidence. All C–G, production fixture, differential oracle, full matrix/profile/scheduler/release gates remain outstanding.

A/B focused unit rerun: **83 passed**, including both new tests (`ab-unit-final.log`). No production adoption/performance acceptance claimed yet. Preserve the A/B artifact next before accumulator changes. Harness baseline diagnostic still active at this checkpoint; it prints the CSV only at completion.

## Baseline self-comparison and accumulator checkpoint

Baseline self-comparison completed successfully (12 rows, 3 independent pairs per ICP/generic row): identical cycles and effective state. Diagnostic ICP cost 115,227,735, generic 123,575,775 for 16 synthetic watchers / 32 equal transfers; no performance claim because unrelated builds ran concurrently. Raw CSV `baseline/smoke.csv`.

A/B commit: `cdc555b`; immutable debug artifact `observations/cycle-optimisation/ab/event_horizon_debug.wasm` (digest alongside it). Baseline artifact remains immutable.

Working C change: scan functions now own their accumulator and return `CompletedScan` only on success. Page handlers update that owned map directly; no whole-map clone/merge in production. On failure the scan returns only an error; durable page-level pending-admission/cursor publication and explicit archive progress are unchanged. Focused units running in `c-unit.log` (exec 83126). Preserve a C artifact and run failed-page/independent-stream regressions before proceeding to D. Global snapshot timing is unchanged in C.

C focused acceptance: 83 backend units passed; PocketIC `live_page_cursor_is_atomic_across_a_late_malformed_block`, `distinct_stream_failures_do_not_suppress_the_other_stream`, and `admission_at_page_end_survives_next_page_failure_and_matches_on_retry` all passed against the frozen C artifact (`c/artifact.sha256`). Test loader override `EH_TEST_BACKEND_WASM` affects host fixture selection only. No production/debug export added. Individual serial cost rows remain pending a quiet measurement window.

Next D: apply `/home/codexdev/eh-opt-d.py` and copy prepared independent oracle `/home/codexdev/eh-polling-differential.rs` to `canisters/event-horizon/src/polling_differential.rs`; run focused units. Oracle uses a linear retained-vector algorithm, independent threshold comparisons, deterministic seeds, page boundaries and invalidation. These preparation files are outside Git; preserve final versions in repo before relying on them.

## D checkpoint

C commit `cc5ce45`. Working D adds `CachedInterests`, coupling page-local watcher lists and optional record-high Nat in one cache entry. Existing overlay invalidation removes the entry, resetting the summary. Empty/singleton lists allocate no summary Nat (singleton walks remain cheap). Only watcher matching is skipped; each block still decodes/validates, advances normal activity and handles admission.

`d-unit.log` (exec 77865) runs the new independent linear oracle over 32 seeds × 512 steps, 300 identities × 4 subjects/accounts, page resets, effective threshold reductions, 257/4,097/32,769-bit values and indices near u64::MAX. The oracle does not use `record_match` or `Subscription::matches`. Seed/step are emitted on failure and deterministically reproduce a trace. The threshold-invalidation and 1,024×256 operation-count tests have already passed; complete oracle still running at this checkpoint. Preserve D debug artifact before E–G edits. Paired performance acceptance is pending, not claimed.

D correctness checkpoint passed: **86 backend units**, including all randomized oracle traces (238.50 s). New installed-Wasm `smaller_and_equal_transfers_after_threshold_reduction` passed (16.83 s), using two watchers so record-high filtering actually applies. Existing 1,024×256 count assertion passed (1,024 relation evaluations instead of 262,144); this is not a total-cycle saving claim. D artifact is frozen under `d/`. Individual paired costs remain pending.

Next E script `/home/codexdev/eh-opt-e.py`: make shared snapshot lazy before first live block/admission; generic globals after successful live observation and before independent admission. Production membership writers are confined to the guarded polling lane; alias/pricing/funding lanes do not write globals. Preserve last-live/admitted-at rules. Promote stable-map count helper for daily health. Remove source-location/count static assertions forcing eager traversal, retaining interface/order checks and exercising admission-in-first/last-block PocketIC regressions. Remember to make `shared_globals` parameter mutable for lazy initialization.

E working checkpoint: native real-page traversal test passed at global counts 0/256/1,024, with zero visits on an empty page and exactly one snapshot over two live pages. Direct map count adds no visits. Test-only traversal counter is cfg(test), excluded from debug/production Wasm. `e-unit.log` still contains the longer oracle run; `e-traversal-unit.log` is the new counter test. E debug build active (log `e-debug-build.log`); preserve it before applying prepared `/home/codexdev/eh-opt-f.py`. That F script is NOT applied yet.


E artifact frozen under `e/` (digest alongside). Five serial eligibility regressions are active (exec 49173; also inspect current processes/logs): `shared_global_admission_block_does_not_self_wake`, `shared_global_admission_matches_later_same_poll_activity`, `shared_global_activity_before_admission_is_not_retroactive`, `archive_only_progress_does_not_poke_a_global_subscriber`, `distinct_ledger_admission_is_not_retroactive_within_a_poll`. Logs `e-<test>.log`. Full E unit oracle run may still be active; the new traversal test passed separately. F/G scripts remain outside repo and UNAPPLIED at this checkpoint. G requires adding write-counter/no-op pending-improvement tests after application; no financial journal changes authorised.

## Remaining measurement/release work (not satisfied by current checks)

- Paired harness currently measures synthetic-recipient debug execution and exact effective DebugState only. Extend decoded real notifications, request arguments/counts, large amounts/memos, all admission/alias variants, growing/full accumulators/irrelevant tails, and baseline→candidate upgrades (including pending CMC/surplus). Existing historical fixtures remain intact. Debug synthetic oneway acceptance is not remote delivery.
- Add fixed-principal production setup: PocketIC supports `create_canister_with_id` with NNS subnet enabled. Seed identical fixture state using baseline debug Wasm configured to the actual fixed trust anchors, then argument-free upgrade to the genuine baseline/candidate production artifacts. After settling initial scheduler lanes, measure a controlled scheduled poll; optionally upgrade back to debug only AFTER cost measurement to inspect effective state/cursors. Keep setup, upgrades, pricing/funding credits and observation queries out of poll attribution; disclose rent/timer-lane costs if not separable. Genuine real-recipient cases must verify received notifications. Do not label local unshrunk smoke rows canonical.
- Individual preserved artifacts: baseline, ab, c, d, e. Queue serial paired costs after no other builds/benchmarks are active; do not use the contaminated baseline self-comparison as headline evidence. At least three independent samples, medians/ranges and worst rows, cold/warm separately.
- No instruction instrumentation exists yet. Pinned CDK exposes message `instruction_counter()` and call-context counter; do not subtract message counters across awaits. Any added instrumentation must apply identically to baseline/candidate and be excluded from canonical Wasm. Peak heap is currently unavailable; status memory is settled total/stable, not peak.
- Local host `ic-wasm` is **0.11.0**, whereas canonical is **0.9.7**. Profile experiments must use the pinned transform (e.g. extract `/opt/ic-wasm/bin/ic-wasm` from the unchanged cached canonical Docker builder). Do not quietly use the host transform or change the release pin.
- Compiler experiment not started: baseline z and refactored z before candidate s/2/3; same Rust 1.94.1/dependencies/LTO/codegen/panic/target settings, pre/post-shrink sizes, predeclared representative screening rows and full acceptance for winner. No profile choice made.
- Scheduler: fixed-work rows are not daily burn. Run unchanged scheduler with identical time/funding/traffic; record polls, lag, tiers and actual burn. Recalibrate cadence model from candidate costs; label projections/latency assumptions. Continuous may spend savings on more polls; no backoff or policy changes.
- Higher-complexity disposition still pending explicit evidence: retain exact-route Historian calls by default (alias/price/error/external-call observations make skipping unsafe); consider threshold index only if increasing workloads justify it; retain page-local caches/lazy globals unless bounded cross-page invalidation proven. Stable replacement and financial journal batching are not authorised.
- Final full gates, all required tests/security, actual canonical backend hash, frontend pin/bundle/manifest, committed-source two-build reproducibility and population, remote CI status are ALL pending. Do not claim a release candidate yet.

E acceptance checkpoint: existing 86 backend tests passed, plus the new traversal test separately (87 total cases now present). All five installed-Wasm eligibility/archive regressions listed above passed. No source-location assertion forces eager registry enumeration anymore; timing coverage resides in those tests plus the real-page visit counter. E is ready to commit; individual costs still pending.

## F checkpoint

E committed as `da43560`. Working F uses one optional batch-owned Candid buffer only for `global_activity=true,matches=[]`; ordinary typed encoding remains for specifics/mixed. Each recipient retains its own bounded-wait oneway Call, immediate actual `get_cost()` reserve check, no attached cycles, and the same attempt counters. Unit `f-encoding-unit-final.log` passed exact typed-byte/decode equivalence, one encoding across 1,024 lookups, pointer reuse, and non-global/specific exclusion; counter is cfg(test), absent from Wasm. F debug artifact is frozen under `f/`.

Three F installed-Wasm checks are now active (inspect `f-*.log`): canonical/generic global-only Index callbacks and 1,024-recipient reserve recovery. Do not apply G until F is committed after these checks. Prepared G files: `/home/codexdev/eh-opt-g.py`, `/home/codexdev/eh-relation-keys.rs`; UNAPPLIED.

Host-only `Env::with_anchors` supports production fixed ICP/Historian/CMC/Faucet principals, NNS subnet, deterministic initial time, and freezing threshold zero for controlled test liquidity. Existing fixture callers still use the old path unchanged. This setup support has not yet been measured or accepted as a production-path harness; add an actual uninstrumented production-Wasm scheduled-poll test (and post-measurement inspection) before making any canonical measurement claim.

F acceptance checkpoint: exact encoding/count unit passed; both canonical ICP and generic ICRC-3 global-only Index tests passed, plus `global_fanout_reserve_skips_still_attempt_all_recipients` (1,024 recipients). F is ready to commit. Broader mixed/max payload, actual-cost near-reserve and complete regressions remain final-gate requirements, not replaced by these focused checks.

Next G: apply prepared relation-key/context patch; add isolated native stable-write counters/tests and pending-improvement/no-op invalidation tests. Also remove the unnecessary Nat clone in `SubjectSubscription::account` via a pure account-for-controller helper. Keep unchanged aliases fully validated and always update alias metadata timestamps/revision. Do not touch financial/cursor/price metadata writes. Record that G's cost delta includes completing A's derivation-key reuse.

## G working checkpoint (not yet accepted)

F commit `7cc440e`. G is applied in the working tree. `RelationContext` holds only operation-immutable mode/governance; `RelationKey` reuses unchanged existing AccountKey/ExternalKey encodings through staging, cache invalidation, and publication. Pending duplicates compare against effective pending improvements and retain them; unchanged candidates do not invalidate watcher generations. Publication suppresses unchanged self/external values, repeated global membership, and identical alias bindings. Alias metadata publication remains unconditional; complete snapshot validation is untouched. No financial/cursor/pricing journal write suppression.

`SubjectSubscription::account` no longer clones its Nat-bearing subscription to substitute the controller; both self/subject use a pure identical account derivation helper. Internal debug/test wrappers are cfg-gated because production now uses keyed helpers directly. Unused private merge method removed; monotonic behaviour is covered by existing merge tests, new effective-state/write-count tests and pending-overlay tests.

Initial G check found unused wrappers (now cfg-gated). Initial G unit compiles failed on ambiguous Nat RHS `.into()` in new assertions only; corrected to explicit Nat. Current focused rerun: exec 96030, `g-unit-corrected.log`; intentionally excludes the already-passed long randomized oracle until the next full acceptance. New tests count actual map inserts (`cfg(test)` only), verify alias timestamps/revision still advance, and preserve a staged lower threshold/cache generation across later no-op duplicates. Do not claim G passed until reading this log. Build/freeze G artifact, rerun D invalidation and aliases/upgrade/duplicate admission regressions next; then do serial paired costs and reject losing variants if needed.

G focused units passed: 89 passed, long randomized oracle filtered once (already passed before G; still required in final full run). Strict backend all-target Clippy passed (`g-clippy.log`). G debug artifact preserved under `g/`; three serial installed-Wasm checks now active: smaller/equal post-threshold invalidation, full alias paging/cache/upgrade regression, cross-subject exact-route/threshold/callback regression. Logs `g-*.log`; no G commit until these are checked.

Production fixture work: genuine baseline companion copied to `tests/fixtures/event_horizon_81c5a0d_production.wasm` with reviewed canonical hash, provenance appended. `production_fixed_work` is added to the host harness but NOT executed/accepted yet. It seeds state with the verified baseline debug module against fixed trust anchors, upgrades to uninstrumented production, settles initial timer lanes, and measures one scheduled Standard poll after 600 s with freezing threshold zero on both variants. After cost capture it upgrades to baseline debug for cursor/state inspection. Default candidate is local unshrunk smoke only; `EH_OPT_CANDIDATE_PROD` must point to final canonical bytes for canonical claims. Ordinary/no-memo and real global-only callback cases, ICP/generic, three pairs each. Cost may include storage accounting for the identical 600 s interval; do not label it isolated synchronous instruction cost. Validate actual behaviour/setup and reject incomplete scans before using any values. No production export/trust-anchor changes.


G final focused checkpoint: strict backend all-target Clippy passed; installed-Wasm smaller/equal invalidation, aliases paging/cache/upgrade and cross-subject exact-route/callback tests all passed (`g-rerun-*.log`). Initial G PocketIC attempt was a host compile failure in the new production helper (moved reference snapshot), corrected before execution. It was not a failed behavioural assertion. Canonical baseline production fixture and host production helper are being committed with G; actual production run is still pending.

Pinned transform extraction succeeded: `observations/cycle-optimisation/pinned-tool/ic-wasm --version` is 0.9.7, digest in `ic-wasm.sha256`. It was extracted from the unchanged pinned Docker tool stage using the baseline archive. This generated no release artifacts and changed no canonical recipe. Docker extraction exec 82463 should be reaped/checked if needed. Use this binary, never host 0.11.0, for screening after `ic-wasm shrink`.

## Measurement transport and new baseline-upgrade checkpoint

G is committed as `7425e22`. Production smoke first attempt had PocketIC HTTP IncompleteMessage (`g-production-smoke.log`), not an assertion failure. Diagnostic rerun reached all six ICP ordinary variants and started global setup without assertion failures; it was intentionally interrupted (SIGTERM to this task's test runner) before the default server hard TTL could invalidate the large combined run. No complete CSV from either attempt; neither counts as a performance pass. Other workspace builds/benchmarks were still present; they were not touched.

Harness now supports complete per-case/per-sample pairs in fresh processes (`EH_OPT_CASE=name:blocks:watchers`, `EH_OPT_GENERIC`, `EH_OPT_SAMPLE`), refuses output overwrites, and the resumable `tools/run-cycle-optimisation.py` preserves independently accepted shards plus input/artifact/harness digests. Performance tests require the host-only `optimisation_benchmarks` feature so full regression discovery does not run a long benchmark matrix as an ordinary ignored scenario. Behavioural upgrade/invalidation tests remain in normal discovery. Fixture batch append avoids thousands of setup ingresses; measured poll input/order is unchanged.

Identical synchronous instruction instrumentation is generated into isolated Git archives only by `tools/instrument-cycle-optimisation.py`. Actual production trust-anchor path is used, with test-only metric exports in those shadow artifacts. No root runtime feature/module/export was added. Baseline instrumented build succeeded (0.9.7 shrink still pending); candidate build active in `instrumented-candidate-build.log` (exec 45399). Counters cover synchronous spans; admission scope explicitly ends before Historian await. Nested phases overlap, observed linear heap pages are not a claim of live/peak allocation, and instrumentation overhead must be compared separately. Never use these artifacts for release or headline uninstrumented cycles.

New reviewed-baseline upgrade test is active (exec 19560, `reviewed-upgrade.log`): starts from verified 81c5a0d debug bytes, seeds self/external/global relations, permanent aliases, lowered thresholds, both cursors/pricing, planned surplus and notify/transfer-pending financial journals, upgrades to frozen G, exercises duplicate declarations/matching and upgrades again, then recovers exactly one surplus. Verify assertions before accepting. Existing 44d83c0 history remains intact. A host compile check passed before the final additional watcher edit; inspect current compile/run result.

Next: preserve candidate instrumented artifact after build; run a single complete production pair via filtered harness (3 shards independently for headline). Then run serial baseline→individual cumulative debug measurements with runner. No headline evidence/release gates complete yet.

Reviewed-baseline upgrade rerun **passed** (`reviewed-upgrade-warm.log`, 146.02 s): all four ICP/generic × notify/transfer-pending cases, complete state/alias/subscription preservation, unchanged pending planned surplus after exercising duplicate matching, second upgrade, and exactly one recovered surplus. Initial run failed with connection refused during a 98-second mock ICRC-3 build after server startup, not an assertion. Env now compiles all required mock fixtures before creating its first server. No retries, execution-limit change, or assertion weakening.

Attribution artifact freshness check caught identical baseline/candidate bytes due shared Cargo target freshness with relocated source archives. Candidate package was explicitly cleaned/rebuilt; correct distinct shadow digests now recorded in their receipts: baseline `edfe500e7c4b67824b57495c4f8003de0e488eaa9bfacd793999361725edb95b`, candidate `52b45dd0bd5fa8671e1039fd91567cac3743207cd511071568bbe0269804757f`. No measurement used the rejected mislabeled file. Reproduction instructions require separate target directories.

Fresh security scan `prototype-security.log` **FAILED OSV**: new GHSA-qwgh-2vcv-g2f7 flags locked block-buffer 0.10.4 through digest 0.10.7/sha2 0.10.9 (also Candid/IC certification transitive). Cargo audit, deny and npm sections preceding OSV passed; verify exact log statuses. Primary maintainer advisory https://github.com/RustCrypto/utils/security/advisories/GHSA-qwgh-2vcv-g2f7 specifies affected `>=0.11.0,<0.12.1`; OSV's imported https://osv.dev/vulnerability/GHSA-qwgh-2vcv-g2f7 has introduced `0`, contradicting it. Local 0.10.4 uses separate `pos: u8`, not the vulnerable inline cursor; immutable crate source and lock digest can be reproduced. Public OSV JSON snapshot under `security/`. Treat as downstream metadata mismatch/security release blocker, not a passed gate. No exclusions or dependencies changed to hide it. Do not broadly migrate sha2/Candid to appease this record.

Compiler screening recipe and representative matrix are now predeclared in `cycle-optimisation-experiments.md`; standalone script preserves raw/shrunk bytes and pinned 0.9.7 audits. NOT run or selected yet. Workspace Clippy exec 42189 still needs final status. Next: commit harness/evidence tools with manifest after Clippy, then serial per-pair measurements; do not silently call current security green.

Workspace strict Clippy passed (`harness-workspace-clippy.log`, 2m10s). The upgraded fixture passed without retries after moving compilation before server lifetime. Investigation is tracked in `block-buffer-osv-investigation.md`; it does not change security policy. Benchmark/instrumentation/profile support is ready to commit as a test-only checkpoint. Runtime remains G; profile selection, serial complete performance rows, full acceptance and final canonical/pin gates remain outstanding.

## Fixed-work settlement investigation — active, no headline claims

Test-support checkpoint `e9ac890`. Initial serial pilot completed three equal256×1024 ICP pairs with exact identical values in all samples: baseline immediate debit 3,274,441,972,797, candidate 3,272,710,749,963. These are far above historical ~7.8B. Current 30-tick-only settlement did not prove expiration/refunds for unroutable synthetic recipients; do NOT label those rows settled net consumption or use their percentage as headline. Runner 417560 intentionally stopped before further matrix expansion; its current child (422084 at checkpoint) may finish independently. Preserve `g/pilot-shared` diagnostic rows/logs; no `complete.csv` accepted for the full matrix.

New harness records both immediate and timeout-settled debit, advances identical debug fixtures by 301 seconds for synthetic send cases (ordinary bounded-wait timeout), then requires ten quiescent rounds with unchanged sender balance. Storage accounting over that interval is explicitly included; fixtures' autonomous schedulers remain off. No production call, timeout, retry or scheduling policy changed. Run one filtered pair before expanding again. Runner resume identity now binds artifact/harness/mock/fixture code rather than arbitrary Markdown diffs, so progress-note updates do not invalidate performance inputs.

Settlement diagnosis resolved: delayed setup debit, not dispatch. Timeline (`settlement-timeline-rerun.log`) showed an identical ~3.240T late debit with one real or one synthetic recipient, after initially settling the actual call at ~19M. Moving fixed time before instance/canister creation and settling fixture setup for 60+10 quiescent rounds removed it. The corrected single probe (`g-setup-settled-probe.log`, `g/setup-settled-probe.csv`) reproduces historical baseline scale: immediate 7,834,465,152 baseline vs 5,997,300,605 candidate; including identical 301-second storage interval 7,846,806,459 vs 6,009,642,312. Both attempted/accepted 1,024 callbacks and exact state matched. This is one sample, NOT headline acceptance yet. Preserve old rows as rejected setup-contaminated diagnostics; do not treat the 3.24T as a production fee, receiver failure or true poll consumption.

`Env::with_anchors_at` adds host-only optional initial clock; existing callers delegate with None. Paired fixture uses it before creation and verifies setup quiescence. Production fixed anchors already set initial clock before creation. Post-poll costs retain immediate and timeout-settled columns; storage interval is disclosed. Diagnostic timeline is feature-only (not normal integration discovery). Initial timeline host compile failed on fetch-log caller argument (Option vs Principal), corrected before execution; no behavioural failure.

Next after committing this fix: rerun pilot three complete samples for equal/increasing/unique cases, ICP/generic serially. Current completed source support checkpoint before this fix was `e9ac890`; runtime remains frozen G `7425e22`.

Working expanded paired harness (NOT yet validated/committed): adds request counts, separately measured Ledger/Historian/CMC/recipient cycle deltas, exact typed-payload byte calculation, initial cursor and plain numeric memory columns. New rows cover zero/random amounts, 1/16 watcher cases, unique watched and unwatched destinations, 4-KiB irrelevant memos/wrong-source/wrong-destination payouts, and generic 257/4,097/32,769-bit amounts. `growing` fixtures actually admit 512 targets to one real subscriber, fill the retained first256, process overflowing candidates/irrelevant trailing pages at 1/2/16/64 pages, update retained target0 in the last block, and compare every exact decoded hint plus real receipt. Start cursor offsets include setup admission blocks rather than assuming zero. Fixture setup and quiescence remain outside cost capture. Compile exec 80230 (`expanded-benchmark-compile.log`) needs inspection. Before launching a long matrix: run one growing2-page ICP pair and one huge-Nat generic pair, verify baseline/candidate equivalence and ordinary execution limits; fix any host compile/fixture assertion failures honestly.

## Expanded boundary acceptance and final borrowed-index completion

Expanded growing512/512 ICP pair passed (`g-growing-probe.log`): exact256 decoded hints at a real subscriber, first-retained targets0–255, update retained target0 in last block after discarded competing targets, request counts identical. Sender including storage: 269,966,877 baseline vs252,216,301 G. One sample only.

New generic 32,769-bit ×32transfers×1,024recipients stress case exceeded **baseline** ordinary40B single-message instruction limit (`g-huge-probe.log`), so it produced no accepted row and no saving claim. Preserve/report this resource boundary; no production limit or service was lowered. Stress measurement envelope uses16 recipients for32,769bits; its bounded paired probe passed (`g-huge16-probe.log`): 1,681,261,693 vs1,654,593,396 including identical storage interval; all16 sends accepted, exact state/call counts equal. 257/4,097-bit cases remain1,024 recipients until tested. No raised execution/decoder limits.

New controlled-await baseline/candidate test **passed** (`mutable-input-valid-alias.log`,27.45s). Frozen price10 admission starts a route call, mock pauses, separate pricing advances to5 and additive aliasB publishes, first decision still rejects500M againstold1B requirement; later B.8 observes newalias/newprice and admits; unknownY.9 is consumed without a route request and neverreplayed after publication. Exact destinations/memo bytes, state andrequests match independent81baseline. Explicit limitation: real Historian replicated query cannot itself await. `interleaving_gate` is a mock-only update wrapper around the identical pure response, preloaded only for this exact test; normal mock keeps query interface and performance runs use normal mock. No EH production/debug method added. Initial attempted query pause failed with forbidden ic0_call_new (test-facility error). First update-wrapper run used reserved aliasX and correctly rejected the alias table; corrected fixture to validB without changing validation. Useful failure logs retained.

B audit completion in working runtime: `u64nat` now uses existing `TryFrom<&BigUint>` instead of allocating a clone for each ID/head conversion; exactu64 and error text retained. New domain test includes0,2^53+1,u64::MAX,65-/32,769-bit overflow. Independent oracle now also admits new effective identities/subscribers/neuron targets mid-trace, invalidating generations. Full backend **91units passed**, including all32×512 differential traces (`borrowed-index-full-unit.log`,66.33s). New debug artifact (H) still needs build/freeze and commit; G artifact is immutable.

All-feature workspace Clippy currentlyexec98812 (`expanded-interleaving-clippy.log`) needs status. Official PocketIC15.0.0 binary downloaded inside observations/pocketic15 with receipt/digests, `--version` verified, for explicitly fingerprinted subsequent runs. Pinned compiler/transform/dependency graph unchanged. Runner receipts now cover all mock sources, helper/runner code, Cargo.lock, rustc and optionalPOCKET_IC_BIN digest, with separately logged resume source revisions (Markdown-only commits need not invalidate unchanged inputs). Profile tool also preserves matching shrunk debug companions for its predeclared screening matrix. No compiler choice or canonical candidate evidence yet.

Next commands after Clippy passes: build/freeze H debug (ordinaryz), stage new sources/regenerate+verify manifest/commit this checkpoint; run serial three-sample paired H pilot and full relevantmatrix, then individual checkpoint attribution; perform baselinez/refactoredz/s/2/3 screening serially; genuine production/shadow instruction comparisons and scheduler evaluation remain outstanding, as do final full gates/pin/canonical builds/CI. Security stillFAILED on upstreamOSV metadata mismatch (no exception added).

Working publication harness added in `tests/pocketic/src/cycle_optimisation_publication.rs` (feature-only, runner modepublication): ordinary no-op self/external/global declarations, staged reduction followed by no-op duplicates, 256-wide new/full-u64-endpoint ranges, unchanged/additive alias snapshots with2/4,096entries. Exact state/cursors/call counts and actual global delivery asserted; external threshold probes occur after cost capture. Normal mock retains query methods. Host compile passed. Serial first probes active exec70218 (`h-noop-self-probe.log`, then`h-alias4096-probe.log`) against immutable H; inspect before accepting/committing.

H debug build succeeded: `fdd243557bd3631f1428689dfd68c3fa595f5f58d8fca95c78f28e588dd1e26e` (unshrunkz). All-feature workspace strictClippy rerun passed; initial extra-feature check found an existing needless return in debug-only abort helper, removed without changing its value. Native91 pass predates that debug-only expression formatting; normal production unchanged except borrowed-index conversion.

Production host matrix expanded in working tree (not yet compiled/run after expansion): ordinary/no-memo, real global-only receipt, idle0/256/1,024globals, actual256-target admission, and actual256-hint specific receipt. Generic new-range admission writes only ICPadmission cursor, not observed cursor; expected domains encoded explicitly. This remains genuine fixed-anchor production execution with debug-only seeding/inspection outside costs. Do not claim these new rows passed until running them. Source stillnotcommitted; final profile/canonical release and security/CI acceptance pending.

Publication matrix also includes newglobal (one final-block admission, no self-wake) to measure the added lookup versus insertion, not only duplicate-global wins. Summary tool now labels publication/production fixture `entries` separately from debug `watchers`. New production rows are unaccepted until smoke checks; final single-pair runs and three-sample matrix next after committing.

## I — measured no-op regression correction / alias probe accepted

H publication no-op self256 regressed: baseline1,814,916,729 vsH1,842,639,896 (+~1.5%), exact state/calls equal (`h-noop-self-probe.log`). Investigation found G discarded a stable no-op value, forcing repeated key derivation/stable Candid reads for every duplicate in the page. Working I caches the known effective relation and a transient dirty flag in PendingAdmissions. Unchanged records neither invalidate watcher generation nor publish; later lower thresholds retain dirty improvements. Every exact-route call/price/alias read remains before staging as before. Cache is page-local, bounded by existing page×range limits; it adds retained key/value memory in duplicate/no-op pages, compared with G, and must be included in memory trade-offs. No durable field or encoding changed.

I focused native acceptance passed:91 tests +1 filtered long oracle, including a new actual key-derivation counter assertion (256 no-ops →1 derivation), zero writes, unchanged high-water generation and prior staged-improvement retention (`i-cache-focused-unit.log`). Previous full91 included expanded independent oracle; final full oracle still required after all source/profile choices. Strict all-featureClippy active exec37538 needs status; I debug/prod artifacts not yet built/frozen.

Alias diagnostics: mock query-local ALIAS_CALLS mutations are discarded in replicated queries, so that counter cannot supply positive counts. First same-day probe correctly did no refresh; later due probe failed only the invalid positive counter assertion. Fixed fixture explicitly includes E plus4,095 valid aliases, clears its scheduled timer through same-variant argument-free debug upgrade, advances a day and settles accounting before the measured explicit refresh. Metadata proof checks exact successful timestamp/attempt/revision digest, rejecting skipped/failed scans. `alias_calls` raw field is unavailable (blank), `validated_alias_pages` is derived from complete bounded fixture (not claimed a counter). Instrumented shadow builds now count caller-side alias_request immediately before await; publication metric driver checks actual32 attempts separately.

Corrected4,096 unchanged-alias paired probe **passed** against immutable H (`h-alias4096-metadata-probe.log`,93.24s): baseline827,840,139 vsH742,128,749 sender cycles; Historian748,997,341 on both;32 validated pages and exact effective metadata/state equal. One sample only, not headline median. Early preview warm-up Result was initially ignored; now required to succeed with explicit E binding. No assertions weakened to accept incomplete refresh.

Production/per-publication metrics share the same shadow Metric decoder; publication shadows require debug_api,optimisation_metrics (clearly debug configuration), production shadows only optimisation_metrics. Actual production fixed-anchor rows stay separate. Runner fingerprints every included harness source and all mock sources; summarizer requires exactly distinct samples0/1/2. Source and progress stilluncommitted; next afterClippy: build/freeze I, rerun no-op self regression, run baseline-upgrade/invalidation against I, stage/manifest/commit, then serial three-sample matrices and compiler screening. Final gates/security blocker/CI remain pending.

I debug/prod built and frozen. Digests: debug `4971aaf82ed1acaa2b84e2adb7159a0f12ae6b727773b923fc6caaa6129ac7aa`; local production raw `4645abfc0c2bca638af01c8d77050c7c75c6ee893c457fb3737b60e0af75dd7d`; pinned0.9.7 shrunk production `35f72ff8a36acded4506c8b19689b9837ee66120494156f56bf71fc663181474`. Export audit passed11exports/three application queries (`i-production-export.log`). These are LOCAL experiments, not final canonical hashes/pins.

I no-op regression rerun passed (`i-noop-self-probe.log`): baseline1,814,916,729 vsI1,795,023,763 (~1.1%saving), identical256 exact-route work evidenced unchanged Historian debit1,460,758,528, state/cursors. The prototype H regression has been corrected without suppressing calls. SourceI caches known unchanged relations perpage with dirty=false; native counter proves256 no-ops deriveonekey andpublishzero writes.

Genuine fixed-principal uninstrumented production probes passed against canonical baseline and LOCALpinned-shrunk I: `i-production-range-probe.log` baseline191,064,359→I138,573,961; `i-production-specific-probe.log`194,860,396→163,570,385. Range actually publishes256 subscriptions; specific delivers/decodes256 exact hints at realreceiver. Both state/cursors/Standardmode match. Rows include identical600-second accounting interval; both only one sample, not headline acceptance or canonical reproducibility. Canonical source/backend pin unchanged until final profile/build. New explicit caller authentication assertion and topology logging are host-only; compile/Clippy after them stillneeded.

Next: final focused I upgrade/invalidation plus new harness Clippy, stage all sources/manifest and commit (HEAD stille9ac890 before these working changes); run serialthree-sample I debug/publication/production matrices, individual preserved-checkpoint measurements and shadow instruction counts, then compiler profile screening and scheduler evaluation. No full release gates or final CI claimed. Security externalOSV discrepancy remainsfailed/unexcluded.

I focused installed-Wasm acceptance all **passed**: reviewed81baseline→I upgrade/second-upgrade/planned-surplus recovery124.38s; smaller/equal post-threshold invalidation24.91s; controlled mutable alias/price interleaving83.38s. Logs `i-accept-*.log`. Strict all-feature workspace Clippy passed after main formatting and heap-column correction (`measurement-matrix-final-clippy.log`,12.44s). Initial heap-column host compile had duplicate formatting arguments, corrected before measurements; no contract assertion ran in that compile failure.

Heap telemetry now directly records management `wasm_memory_size` before/after; label allocated linear-memory high-water, not live heap. Fixed-work matrix adds firstbootstrap (historical head skipped prospectively), eight repeated empty polls, and archive-only512 cursor progress at0/256/1,024globals. These new rows are compiled/Clippy-clean but NOT yet executed. Raw completed_polls/processed_blocks/fixture_phase distinguish service from cursor-only progress. Core runtime unchanged after I artifacts frozen.

Normalz mocks compiled/frozen (seven packages) under `frozen-mocks-z/receipt.json`; runner `--mocks` and EH_OPT_MOCK_DIR hold recipient/dependency binaries constant across compiler experiments. Controlled-await Hist feature bypasses that override only in its isolated explicit test. Official PocketIC binary is fingerprinted in pocketic15/receipt.json. Before headlines run fresh three-sample rows with these frozen inputs. Sources/manifest need commit now; no other tests/builds remain active at this checkpoint except verify with process inspection. Next exact sequence: stage current14 source/tool/doc changes and new include files; generate+verify manifest; commit; record source in I artifact receipt; run runner debug filtered bootstrap/archive/repeat plus equal/increasing/unique three-sample pilot, then remaining matrix/publication/production. Preserve old diagnostic/failed rows. Compiler screening/production metrics/scheduler/small mixed workload/final full release gates stillpending. No security exception or deployment.

## Committed runtime I / authoritative pilot active

Source/runtime checkpoint committed clean as `57b967b0256f40405f6ed5b79ecf3af15e6be672`; manifest verifies (`i-manifest-verify.log`); immutable I provenance.json now records source/input/artifact digests. Last runtime changes are borrowed u64 conversion plus corrected page-known no-op dirty cache, all A–G preserved. New runtime source does not yet include a potential ICRC-key capacity correction discussed below; do not label any later source as measured I.

Active serial pilot exec84621: `POCKET_IC_BIN=$PWD/observations/cycle-optimisation/pocketic15/pocket-ic python3 tools/run-cycle-optimisation.py debug tests/fixtures/event_horizon_81c5a0d_debug.wasm observations/cycle-optimisation/i/event_horizon_debug.wasm observations/cycle-optimisation/i/accepted-pilot --mocks observations/cycle-optimisation/frozen-mocks-z --case bootstrap:1:1024 --case archive_only:512:1024 --case repeat_idle:0:1024 --case equal:256:1024 --case increasing:256:1024 --case unique_unwatched:256:0`, outer log `logs/i-accepted-pilot.log`. SixICP groups completed3pairs each; generic groups stillrunning. Complete.csv exists only after all36pairs pass. Do NOT change harness/mock/runner inputs during this run. Resume identical command; code/input digests enforced, completed shards immutable.

Completed ICP group medians (3independent samples, exact no spread seen in first groups): firstbootstrap1,024globals42,759,726→36,476,941 (~14.7%); archive-only512cursoradvance1,024globals26,561,669→20,287,939 (~23.6%); eight empty polls1,024globals155,849,315→105,824,693 (~32.1%). No historical/archived blocks processed, no sends, exact state/requests match. Old allocatedlinearheap1,310,720→1,376,256, Iremains1,310,720 in these three rows. These are finite debug-path pilot groups, not final canonical/profile acceptance or daily burn. Remaining group results/summaries pending.

Drafts prepared but NOT linked/compiled/applied: observations/cycle-optimisation/drafts/cycle_optimisation_small.rs (real1/16relations with self/external/neuron mixtures and exact notifications); cycle_optimisation_scheduler.rs (unchanged actualscheduler, fixed660seconds, one assumed second perPICround, same normalized actual funding/freezing0, Standard/Continuous,1real mixedreceiver vs1,024idle globals). Scheduler draft explicitly reports raw window debit vs settled window+postwindow debug-control/drain, inflight/extra polls, lag and tier seconds; any daily projection must be modelled and disclose drain/latency assumptions. Needs compile/fixture validation and ideally refine attribution. Never claim raw pre-settlement debit as burn.

Additional obvious B audit finding for AFTER pilot: AccountKey::from(Account) currently reserves1+owner.len()+32 but writes two header bytes, causing a needless growth; change capacityto2+owner.len()+32 without changing bytes/BOUND. This is not yet applied. Preserve/freeze a J artifact, prove encoding/full-domain tests, measure I→J generic unique/newrange before selecting final source/profile; do not mix it silently into I measurements. Considered worthwhile small fix, not new indexing/storage schema. No cross-page/all-subscriber cache authorised.

Next afterpilot: inspect allcomplete rows and any repeatable regressions, commit updated notes/manifest as needed; implement/measure small capacityfix and link/validate prepared small/scheduler fixtures, then run remaining full/per-component matrices; source/profile z/s/2/3 experiment, production/shadow counts, finalfullgates/canonical backend pin/frontend/two-build evidence/CI remain outstanding. Security scan stillfailed on knownOSV metadata mismatch, no exclusion/dependency refresh, no deployment.

## Pilot complete / J key-allocation follow-up

All36 pilot pairs passed; complete rows in `i/accepted-pilot/complete.csv`, immutable input/artifact/PocketIC/frozen-mock receipts. Three independent samples in every group were deterministic (zero observed range). ICP/generic equal256×1024 savings23.41%/23.34%; increasing amounts0.147%/0.132%; unique unwatched5.35%/0.353%; repeated empty eight polls1024globals32.10%/20.15%. Empty/bootstrap/archive rows matched exact state/calls and never sent a notification. No adverse row in this pilot; it does not replace remaining workload/production/profile/scheduler acceptance.

Working J corrects `AccountKey` capacity from one to two header bytes. Golden encoding/domain test passes for0/1/10/29-byte principals and None/zero/nonzero subaccounts, confirms exact bytes and no buffer growth. No encoding/BOUND/stable ID changes. J debug build and small1/16real self/external/neuron fixture probe active exec53198 (`j-debug-build.log`, `j-small-probe.log`); no performance acceptance until it exits. New small/scheduler include files compile; runner supports their separate modes. Scheduler fixture remains unexecuted and any daily projection is explicitly modelled. RuntimeI artifact remains immutable. Full gates/canonical pins/repro/CI pending; OSV blocker remains unexcluded.

J focused acceptance **passed**:93 backend units including full independent randomized oracle48.39s; strict workspace all-target/all-featureClippy9.57s; mixed1/16relation ICP/generic real-receiver probe74.73s; Continuous1024idle-global ICP/generic probe97.33s. Standard1mixedrealreceiver probe33.03s also passed. All logs `j-*`; initial small probes failed fixture path, settlement, and self-map-only count checks, not runtime equivalence. Corrected fixture verifies every expected external/self/neuron hint at actual receiver. Fixed storage settlement120rounds plus10quiescent retains observed costs. RawJdebug SHA87b97d02562fc5bae51a4e9a317e11c68b87bf07dc37a04eed87ab5a09d04066.

Continuous pilot has660 completed polls in660simulatedseconds on both variants (round-latency constrained), no blocks/sends/lag, same110Tinitial actual liquidity and unchangedContinuous tier. Settledwindow+explicitdebugdrain costs ICP18,762,680,531→14,618,935,157; generic21,952,896,066→18,697,573,584; extra drain lane start1 each. These are single-sample setup probes, not a daily-burn or mainnet claim. Raw pre-drain debits kept separately. No storage/financial/cadence policy changes.

Next: commit J/harness/manifest, preserve Jprovenance; paired I→J key-capacity measurements, then clean committed-source baselinez and candidatez/s/2/3 compiler screening. No profile chosen; final full performance/release/CI gates stillpending. The Iprovenance text was corrected to say unchanged Cargo release-default overflow policy (the previous ignored JSON incorrectly described explicit enabled checks; no build flags actually changed).

## Compiler builds complete / genuine production upgrade acceptance

J runtime committed00c0d63. I→J three-sample isolated generic key-capacity savings:388,352cycles (0.3904%) unique256unwatched,424,093 (0.3606%) new256-targetrange; zero observed sample range. Raw CSVs in `j/capacity-{unique,range}/complete.csv`.

Clean committed-source local compiler builds completed allz/s/2/3 with pinnedRust1.94.1 andic-wasm0.9.7, unchanged locks/LTO/panic/overflow defaults. Independent baselinez reproduces BOTH canonical reviewed hashes279270.../2658a8... exactly. Candidate backend raw→shrunk bytes: z1,565,451→1,431,613; s1,859,094→1,730,626; 2=1,967,044→1,847,228; 3=1,990,877→1,870,733. All backend/frontend export audits passed. Fullsizes/hashes in `profiles-{baseline,candidate}/sizes.json`. No profile selected; do not pin a local experimental hash as final release evidence.

Added genuine baselineproduction→candidateproduction fixed-anchor upgrade tests both **passed**, ICP39.78s/generic43.44s (`j-canonical-upgrade-*.log`). Each preserves both pending financial phases/frozen surplus, aliases/prices/lower self/external/global relations; exercises two matching/upgrade passes; recovers exactly one surplus. Candidate was pinned-shrunk localJz, old production bytes verified reviewed279270...; final canonical rerun still required. Debug81 and44d history tests retained.

Frontend profile screening passed20 certified requests perprofile and exact asset bytes. Raw `frontend-profile-screen.csv`, diagnostic `frontend-profile-aggregation.csv`; original attempt overflowed becausePICdefault creation balance was omitted from initial-funding arithmetic, corrected to capture actual balance. Status query estimates report13calls for20known requests and remain unchanged after3epochs, matching spec's permitted delayed/noisy aggregation. Treat instruction estimates as coarse raw data, not exact20-request costs. Profile3 frontend8.35%larger thanz; install/certification12,460,593,460→14,223,563,145(+14.1%one-time); statusidle1,169,624,340→1,177,639,374cycles/day(+0.685%); allocatedheap3,997,696unchanged. These are single-sample screening costs; backend cycle comparisons decide profile trade-off. Staticassetcontent/exports unchanged.

Next: commit host-only support/docs/manifest afterClippy; run48serial predeclared compiler screen pairs (onescreening sample perrow; final winner needs three-sample full matrix) via ignored `observations/cycle-optimisation/profile-screen.py` with explicitPocketIC/frozenmocks. Script generated beforefirstmeasure and records source/artifact/harness digests. Main runner's ordinary traffic now includes1/2/16/64uninterestedpages for fresh model coefficients, plus canonical receipt field. Final source/profile, fullmatrix/component/attribution/scheduler/release/security/CI gates remainpending. No deployment.


## Profile screening accepted / provisional profile2

Host-support commit8802517; all48predeclared one-sample profile screening pairs passed (`profile-screen/COMPLETE`, logs and passedCSV receipts). Candidate2 versus reviewed baseline: debugequal256×1024 ICP25.34%/generic25.27%; increasing7.07%/7.05%; uniqueunwatched17.12%/13.86%; idle1024globals33.95%/22.16%. Genuine production ordinary256ICP119,843,076→89,725,994(25.13%); generic124,305,728→112,025,529(9.88%); realglobalonly1.21%/1.19%. These are screening samples, not final three-sample headlines. Compiler/source effects stay separated by candidatezrows.

Profile2 chosen provisionally over3:3 has marginal wins in idle/equal/genericordinary, but is slightly worse in increasing/ICPordinary/unique, and adds1.27%backend/1.54%frontend bytes over2. No universal or weighted-average claim. Compared withz,2 adds29.03%backend/6.71%frontendsize. Frontendone-timeinstallation+11.67%, statusidle+0.551%; actualcertifiedHTTPassets/allocatedheap unchanged. This recurring storage/installation trade-off must accompany CPU claims and fullacceptance. Original shrinkpass retained; no transform experiment is needed because measured finalshrunk modules show compiler gains.

Tracked raw screening CSVs/size/input receipts under `docs/codex/cycle-optimisation-data/`; runtime sourceofartifacts00c0d63, harness8802517. Cargo profile changez→2 is the only new production-source setting at this step. Pin is intentionally stillreviewedoldbackend until actual canonicalbuild; localprofile2hashf93cd9... is experimental, not final release evidence. All locks/toolchain/baseimage/otherflags remainunchanged.

No build/benchmark remains active afterexec97387 completes. Next: finish focused host daily-scheduler/model-input/reserve-wrapper support from ignoreddrafts, test it; then freeze support/manifest and run serial finalthree-sample/full/per-component/attribution matrices. Drafts are NOT yet applied: scheduler_daily(two tuple/format checks needed), prepare-model(adapter needs rawsenderbefore/afterCSVfields +global0case), sender-reserve-probe +hosttest(generated shadowonly). Final release/CI/securitygate remainspending/unexcluded; no deployment.


## Final measurement support / full-day acceptance

Runtime/profile checkpoint9e304e1; currentworking changes are host-only fixtures,
input adapter and generated shadow-wrapper tools. Full-day Standard idle1024ICP
probe **passed**,53.00s (`daily-standard-probe.log`/CSV):144polls,24funding runs,
onepricing run, complete dailyaliasrefresh, no blocks/callbacks/lag, same6Tactual
initial funding and Standard tier for86400seconds. Rawwindow8,095,062,841→
7,140,789,347; settledwindow+explicitdebugdrain8,150,526,460→7,196,338,722.
This is oneprobe, not a final three-sample headline. Actual daily saving~11.7%
is smaller than34%fixedempty-poll saving because storage/maintenance/timer costs
remain. 600-second steps/20same-clock rounds represent negligible message latency;
660-second Continuous fixtures keep the separate one-second-round assumption.

Fixed-work CSVs now retain actual senderbefore/afterbalances, and global0single
activity provides a fresh zero-recipient fanout control. Model adapter uses these
actual measured values, never stale example coefficients, and clearly labels its
ordinary-polling-only projections/unknownmaintenance rather than claiming full
runway. Generated reserve wrapper/hosttest are prepared but NOT yet built/run;
shadow templates add no rootproduction feature/module/export. Check boundary
method generation andactualcost/equality before accepting. Mainfinalmatrix is
NOT yet started. Nextcommit support/manifest afterClippy; build baselinez,
candidatez and candidate2 production+debugshadows in distinct targetdirs; run
near-reserve shards and finaluninstthree-sample/component/scheduler/metrics
matrices serially. Preserve failed/incomplete diagnostics. Final canonical
backendpin/frontend/repro/fullregression/security/CI remainpending; no deployment.

## Reserve boundary and adverse-case acceptance / final matrix next

Runtime/profile unchanged4cd89c8 (production codeJ/profile2). All six fresh shadow
builds succeeded, distinct target directories, full source/tool/artifact receipts in
`final-shadows/{baseline-z,candidate-z,candidate-2}/`. These contain extra counters
and probe exports and are NOT releases. Genuine uninstrumented comparators remain
`profiles-baseline/z/` and `profiles-candidate/2/`; frozen normal mocks unchanged.

Actual sender reserve checks **passed** for globalcold/warm,1×257bit,
256×4097bit and64×32769bit envelopes on baseline/candidate. Typed/raw CDKcosts
identical; actual suppliedliquidity at reserve+cost-1 rejects, equality/+1 accepts,
remainingliquidity is precisely1T/+1 immediately after acceptance. Actual standard
receiver authenticates sender and verifies full decoded payload digest. RawCSV/logs
in `reserve-boundaries/`; successful shard durations83.92s(global),74.12s(64large).
The initial256×32769 wrapper exceeded baseline ordinary40B instructions before
producing accepted evidence (`reserve-256-32769.log`, source4cd89c8). It performs
three encodings plus decoding, so this is NOT proof of a production-sender limit.
The accepted policy-probe envelope uses64large hints; full256×32769 ordinary
receiver-decoding regression is unchanged. No limit, decoder setting, hint or
production path was reduced. Keep this explicit measurement limitation in report.

D's isolated C→D adverse workload pairs all passed three samples, exact zero
observed spread (`components/d-unique/complete.csv`): ICPincreasing+3,358,969cycles
(+0.04247%), genericincreasing-1,062,955(-0.01339%); uniqueunwatchedICP+24,819
(+0.03802%), generic+9,927(+0.00996%). Allocatedlinearheap unchanged in all four
rows. Extra summary bookkeeping/oneclone for increasing multiwatchers explains
small overhead; retain simple bounded page-local design for hot equal/decreasing
workloads rather than adding pointer/allocation machinery for<0.05%regressions.
Document these real regressions, not as noise or hidden in combined totals.

Mixeddispatch probe **passed** both architectures37.70s:512global-only and512
specific recipients,1024attempts/accepts, identical cursors/state/calls/payloadbytes;
ICP5,856,829,558→5,684,855,651, generic5,865,916,041→5,692,512,726. Oneprobe,
not final median. Added mixedcase to final/per-component matrices. All mainfixture
code now compiled; onlyhostbench envelope/fixture changes since4cd, no runtime
edits. Next strictClippy/static/manifest/commit to freeze finalharness, then run
final fullthree-sample debug/publication/small/scheduler and preserved per-component
matrices serially; instructionattribution comparisons use explicit shadowpaths.
Final fullgates, actual canonicalbackend/frontendpin/twobuilds, freshsecurity and
CI remainpending. SecurityOSVmismatch remainsfailed/unexcluded. No deployment.


## Active final matrix checkpoint

Harness8876833 freeze passed fmt/strictClippy/static/manifest. Serialpipelineexec43215
started full three-sample debug→publication→small→scheduler modes, explicit frozen
baselinez/candidate2 andnormalzmocks/PocketIC15. Output paths and process pointer
in `observations/cycle-optimisation/active-process.json`. Atcheckpoint56pairs passed,
all ICPidle/bootstrap/archive/global/mixed and initialordinary: global1024median
5,744,359,860→5,555,701,382(3.284%); idle1024 19,489,546→12,873,139(33.948%);
eightempty155,865,315→102,997,932(33.919%). These subsetgroups have3samples;
wholematrix acceptance remainspending. Do not interrupt to run concurrent builds.

Adverse Dthree-sample/actualreserve boundary rawtables and shadow source/artifact
receipts are now tracked under `cycle-optimisation-data/`. Source rootprogram
unchanged. Continue metrics/per-component data after pipeline. Prepared ignored
host-only complete-stable-state reader draft is not yet applied/tested; it will
compare logical keys/Candidvalues through the existing pinned stableSDK on copied
PocketIC memory (allIDs0–11), ignoring permitted physical B-tree history changes.
This adds no production method/feature/schema. Needs compile/differential acceptance
before final gates. The task remains incomplete until remaining validation and
security/CI blockers are addressed/reported accurately. No deployment.


## Complete-state and settlement strengthening

The main three-sample pipeline stopped at generic `growing:4096:512` sample0 after 261 successful pair shards. The unchanged rerun failed at the same accounting check. Balance decreased by 12,258,255 near round50 after the 301-second clock advance, then stayed flat through round200 and 20 additional rounds (`growing16-long-drain.log`, successful). This is a harness settlement boundary, not an intermittent assertion or production retry. All fixed-work/publication/small/production/scheduler final drains now use200+20; initial failed data is retained. No old partial matrix is labelled final.

Host-only stable reader uses pinned SDK structures on a memory copy and compares logical records, not physical B-tree history. First compilation found borrowed-key ownership errors; corrected explicit host clones. The first execution found region8's deliberately real balance sample differed after different cycle consumption. The fixture now supplies150T to the unchanged funding observation, below both real seed samples, and compares all fields exactly. Both complete tests passed. Test-only SDK dependency updates Cargo.lock by one dependency-name line, no versions/packages.


## Strong-settlement matrix started

Checkpoint `8c65325` passed all six largest growing64 pairs (ICP/generic, three independent samples), including exact256hint contents and post-overflow retained updates. Fresh queueexec7772 uses immutable baseline shrunkz/candidate shrunk2 and frozen mocks; `active-process-v2.json` and `logs/final-v2-queue.log` identify the live phase and command. No skipped or failed shard enters completeCSV. Queue stops at the first failure; preserve logs and investigate before changing anything. Current tracked helper/profile/runtime source must remain fixed; documentation-only progress commits do not change the receipt's harness identity.


## Serial release-gate handoff queued

Main measurementexec7772/PID759516 remains active (final-v2 debug at checkpoint: more than100pairs passed, through ICPzeroamount/shared traces; no new failure). Reproduction scripts are now tracked at `tools/run-cycle-optimisation-{components,attribution}.py` (`fef1972`); the queued ignored scripts are identical copies. Do not change measured inputs.

Waiting exec **97871** runs `observations/cycle-optimisation/run-prepin-gates.py`. It performs no build/test until main queue's attribution COMPLETE and passed pointer are present. A failed/interrupted measurement aborts the handoff. It then requires clean source and serially runs exact fmt, strict Clippy, xtasktest_unit, xtasktest_all, security commands; no benchmark overlaps a build. Native fixtures are rebuilt from current source (performance artifact overrides explicitly removed). All normal integration scenarios including Index/maxpayload and the new independent stable comparisons remain discoverable. Any failed gate stops; no security failure is waived. State/logs: `prepin-gates-state.json`, `logs/prepin-gates-queue.log`, `logs/prepin-{fmt,clippy,unit,all,security}.log`. Keep tracked source clean while these gates run; write draft analysis only under ignoredobservations. Final frontend-pin changes later need their affected Node/certified-asset/static/manifest/export reruns and canonical checks.


## Final-v2 ICP fixed work complete

All45ICPfixed-work groups passed3independent pairs each (135pairs), no overall candidate regression. Source/profile/mocks remain fixed. Lowest reduction1.6429% (`global:1:1`,31,409,599→30,893,574cycles); highest36.7097% (`backlog:16384:16`,5,403,239,535→3,419,727,870). Equal256×1024:7,846,795,403→5,858,110,163(25.3439%); increasing256×1024:7,922,889,782→7,362,979,938(7.0670%). These are uninstrumented shrunk debug fixed-work sender debits with the declared settlement interval, not canonical-production or daily-burn claims. AllICPgroups had zero observed sample range; mainnet uncertainty is not estimated from that.

Queueexec7772 is now in generic debug cases; maincompleteCSV waits for all279pairs. Post-gateexec97871 remains waiting, with no build/test overlapping measurements. Current runtime stillJ/profile2; new commits only support/evidence. Continue reading active-process pointers/logs. Do not write tracked data/report while automatic unit/all gates run; draft underignoredobservations until they finish.


## Remaining canonical production-path coverage

Before claiming final principal-win production evidence, read `observations/cycle-optimisation/drafts/canonical-production-extra-plan.md`. Current genuine production helper's default seven cases exercise immutable principal parsing, idleglobals, realglobal1, newrange and specific256, but not repeated shared-account/globalencoding or many-page copy costs. Plan extra feature-only host cases after all current measurements finish, with real receivers and existing bulk admission APIs, no production export/anchor change. Preserve old hostcohort8c65325 receipts; newcanonicalhost fixture gets a separate cohort, no within-run edits. Normal runtime/profile stays fixed. Rerun affected featureClippy/static/manifest and focused fixture proofs after adding benchmarks. A full1024realreceiver smoke may be one clearly labelled sample; principal headline rows still require3samples. Synthetic global9byte/external10byte debug IDs occupy different spaces; do not assume they are creatable PICcanisters or confuse counts. Never accept fewer real deliveries as a saving. Existing unused productionhelper repeatidle/bootstrap/archive array names lack fullsetup and were never defaultselected; do not claim them as canonicaltested.


## Future component coverage strengthened before first run

While generic archive-only fixed-work rows run, add `ab-admissions` publication cases (new/full-width256range, no-opself/external, thresholdlower; botharchitectures3pairs) and include increasing/unique-unwatched in D's new200+20settlement cohort. These fill user-required attribution gaps, rather than relying on unrelated cumulative admission rows or the older adverse receipt. Both tracked and queued component scripts changed identically before their first measurement. Mainrunner/harness/mocks/artifacts are unchanged; active final-v2 fixed-work inputs remain frozen. Component script now has8steps; no existing acceptedcomponentrows overwritten. Native sampler SDK dev-edge remains the only lockfile difference, no dependency version changed.
