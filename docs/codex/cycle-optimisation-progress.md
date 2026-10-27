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
