# Cycle optimisation — resumable progress

Baseline: `81c5a0d72ff188f5ee9f412deb20566a12307642`; clean tree, no existing build/benchmark processes at start (2026-10-27).
Independent detached worktree: `/home/codexdev/src/Event_Horizon-baseline-81c5a0d`.
Evidence root: `observations/cycle-optimisation/` (ignored raw artifacts/logs).

## Status — current checkpoint

- Baseline canonical/debug artifacts preserved and independently verified; baseline self-comparison passed (diagnostic costs only).
- A/B, C, D, E, F committed as correctness-tested prototypes. G focused units (89 plus one filtered long oracle), strict backend Clippy, threshold-invalidation, alias paging/upgrade, and cross-subject exact-route PocketIC checks all passed. G committed as `7425e22`.
- Individual cumulative debug artifacts frozen under `baseline/`, `ab/`, `c/`, `d/`, `e/`, `f/`, `g/`. No measured performance acceptance or release candidate claimed yet.
- Production fixed-anchor helper executed diagnostically; complete split-pair acceptance pending. Full workload expansion, three-sample serial measurements/attribution, controlled differential interleavings and new baseline upgrade tests remain pending.
- Compiler z/s/2/3 screening, scheduler/model recalibration, full regression/security, frontend pin and final canonical two-build gates remain pending.
- No deployment, mainnet mutation, or external repository writes performed.

## Resume

Inspect `git status --short`, `git log -5 --oneline`, and active processes first.
Inspect raw logs under `observations/cycle-optimisation/logs/`.
Current runtime commit is G (`7425e22`); verify working-tree status. Next: run split-pair `production_fixed_work` smoke with preserved baseline and locally built production candidate; label that unshrunk smoke, not canonical evidence. Read remaining-work checklist below before performance claims. Prepared helper patches outside Git have already been applied through G; do not reapply them.
Do not overwrite baseline artifacts. Do not treat interrupted commands as passed.

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
