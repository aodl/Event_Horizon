# Cycle optimisation — resumable progress

Baseline: `81c5a0d72ff188f5ee9f412deb20566a12307642`; clean tree, no existing build/benchmark processes at start (2026-10-27).
Independent detached worktree: `/home/codexdev/src/Event_Horizon-baseline-81c5a0d`.
Evidence root: `observations/cycle-optimisation/` (ignored raw artifacts/logs).

## Status

- Read normative specification, stable-memory contract, subscriber guide, xtask and canonical build instructions.
- Current primary DFINITY guidance being consulted before production edits.
- Baseline canonical build passed in the detached worktree; reviewed hashes reproduced. Debug baseline built and preserved as a tracked test fixture with SHA-256 provenance. Focused serial paired harness compiling (log `harness-compile.log`, exec session 63807). No production edits yet.
- A–G, independent differential oracle, fixed-principal production measurements, compiler z/s/2/3 screening: pending.
- Full regression, security, pin update, canonical two-build gate: pending.
- No deployment or external repository writes authorised/performed.

## Resume

Inspect `git status --short`, `git log -5 --oneline`, and active processes first.
Inspect raw logs under `observations/cycle-optimisation/logs/`.
Next: finish baseline canonical build; preserve hashes/locks/tool versions; implement serial paired harness before production refactoring.
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
