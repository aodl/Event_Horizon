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
