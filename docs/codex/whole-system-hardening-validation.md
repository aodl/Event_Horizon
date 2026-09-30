# Whole-system hardening validation

Validation was completed on 2026-09-30 from audited baseline
`c94c33eb7f8c38da1c6d88c5c99a658e1a8cfbde`. The implementation commits are:

- `c9b83e3e56ab9a097ae0e8a832a6605ef1c18339` — scheduler, frontend,
  reporting, build-context, capacity, and documentation corrections;
- `b8fa2ac33ba50963792f0aa0f32a1820453bc0b4` — compatible exclusive
  Debian snapshot;
- `c5bd9cae4a706aad8c8644b911a8c6f6dfbc8a64` — frontend pin for the
  hardened backend;
- `875d3480af9fc05234b376c6743774c45a5eaf16` — isolated disposable
  builders for the two no-cache reproducibility runs.

`875d3480af9fc05234b376c6743774c45a5eaf16` is the final canonical
artifact-source revision. No executable or build input changed after it.

The externally named `Event_Horizon_whole_system_audit_evidence.zip` and its
four probes were not present in the repository or workspace. They therefore
could not be rerun verbatim. Repository-native deterministic fixtures cover
the same narrow cases. This report does not treat absence of those external
files as proof of any hypothesis.

## Finding A1 — scheduled-worker cancellation

- **Baseline evidence:** Confirmed with the locked `ic-cdk 0.20.1`,
  `ic-cdk-timers 1.0.0`, and `ic-cdk-executor 2.0.0`. A debug-only one-shot
  fault was consumed before a genuine inter-canister await and trapped the
  continuation afterward. Without cleanup, the real timer wrapper retained
  `[running=true, replacement_timer=false]`; after a one-hour advance it made
  no cursor progress. The exact failure was:

  ```text
  Error: scheduled poll did not recover after post-await trap; lane=[true, false]; state=DebugState { ... observed_next_block: 0, polling_mode: Continuous, ... }
  ```

- **Correction:** `canisters/event-horizon/src/scheduler.rs` now uses a small
  scheduler-local guard for the existing polling, funding, and pricing lanes.
  Cancellation drops only the ownership it acquired and installs one delayed
  recovery timer at the existing `RESERVE_RECHECK_SECONDS` interval only when
  no replacement exists. Normal cadence and normal rescheduling are unchanged;
  the cleanup does no inter-canister or financial-state work.
- **Regression:** `scheduled_poll_recovers_after_post_await_trap`,
  `scheduled_pricing_recovers_after_post_await_trap_without_duplicate_daily_observation`,
  `scheduled_funding_recovers_persisted_identity_after_post_await_trap`, and
  `overlapping_scheduled_poll_does_not_release_or_duplicate_the_active_worker`
  passed through the real wrappers. The funding case preserves
  `CmcNotifyPending`, reuses the accepted transfer's memo/block identity, and
  observes one ledger debit rather than two. Existing install/upgrade scheduler
  tests also passed.
- **Remaining limitation:** This demonstrates cancellation recovery for tested
  post-await continuation traps. It is not a claim of recovery from OOM,
  corrupt stable state, every synchronous trap, or a still-outstanding
  guaranteed-response call. The latter remains owned and cannot admit a second
  worker.

## Finding A2 — Reserve Protection entry

- **Baseline evidence:** Source inspection confirmed the scheduled polling
  entry could proceed without first refreshing the persisted cadence mode from
  the current liquid-cycle balance. This was an entry-enforcement gap, not a
  reproduced ledger-client reserve-check failure.
- **Correction:** `canisters/event-horizon/src/scheduler.rs` and
  `canisters/event-horizon/src/cadence.rs` refresh the existing hysteretic mode
  at scheduled entry. Reserve Protection skips `run_poll`, rearms the existing
  reserve recheck, and returns; the per-call client checks remain intact.
- **Regression:** PocketIC proves Reserve Protection at 1.5 T makes no ledger
  call, cursor/admission progress, or false outage log; Economy at 1.5 T still
  polls; funding above 2 T resumes polling; and below 1 T polling remains
  suspended while funding proceeds. These are covered by the four
  `scheduled_*reserve*`/`scheduled_economy_*` tests in
  `tests/pocketic/src/lib.rs`.
- **Remaining limitation:** The tests inject a controlled reported balance in
  debug Wasm. Production continues to read the IC cycle-balance API.

## Finding B1 — frontend cache revalidation

- **Baseline evidence:** Confirmed by source inspection: mutable unversioned
  JavaScript, CSS, and SVG responses used a one-year immutable cache policy.
  The external browser probe was unavailable, so no historical browser session
  is claimed.
- **Correction:** `canisters/frontend/src/lib.rs` serves those assets with
  `Cache-Control: public, no-cache`; HTML retains its non-stale policy.
  `canisters/frontend/public/index.html` makes the one transition to
  `styles.css?v=2` and `app.bundle.js?v=2`. Certification and CSP are unchanged.
- **Regression:** `frontend_transition_urls_are_certified_and_require_revalidation`
  proves old and new URLs differ, both query-suffixed assets resolve, the new
  policy is present, and certified responses verify.
- **Remaining limitation:** This is an HTTP/header and certified-routing
  regression, not an automated real-browser upgrade run. Future changes use
  revalidation rather than a recurring versioning system.

## Finding B2 — stale frontend responses

- **Baseline evidence:** Confirmed as a source-level race: code after an await
  used mutable global selection state and had no request generation. The
  supplied race probe was unavailable.
- **Correction:** `canisters/frontend/public/app.js` and the checked-in bundle
  use a monotonically increasing generation and a captured registry entry.
  Stale success, failure, verification, and final-render/control paths are
  ignored.
- **Regression:** Five deterministic deferred-promise Node tests cover late A
  success, late A failure, selection of a planned entry, identical ledger/root/
  recipient/symbol with different backend and pricing, and current-response
  configuration failure. The latest selection always owns state, and current
  verification failure leaves the builder disabled.
- **Remaining limitation:** Requests are not cancelled; obsolete completions
  are ignored, as intended.

## Finding C — complete fallback failure transcript

- **Baseline evidence:** Confirmed by source inspection: when structured Rust
  failure extraction found no block, only the final 30 lines were retained.
  The external `nocapture` fixture/probe was unavailable.
- **Correction:** `tools/xtask/src/test_runner.rs` retains the complete captured
  transcript in that fallback and appends timeout/spawn/capture/parser
  diagnostics without replacing output. Structured extraction is unchanged.
- **Regression:** `unstructured_nocapture_failure_retains_complete_transcript`
  places an inline assertion with `left: 41` and `right: 42` before 45 later
  teardown lines. The final `Failures:` section retains the assertion, both
  values, last line, known test name, diagnostic, and exact rerun. All existing
  timeout, descendant-pipe, process cleanup, isolation, parsing, zero-test,
  and continuation tests pass; xtask has 19 passing tests.
- **Remaining limitation:** Unknown output remains intentionally unstructured,
  but it is complete.

## Finding D — exclusive APT snapshots

- **Baseline evidence:** The exact pinned base
  `debian:bookworm-slim@sha256:74d56e3931e0d5a1dd51f8c8a2466d21de84a271cd3b5a733b803aa91abf4421`
  (created 2026-02-23) had no `/etc/apt/sources.list`, but had an active
  `/etc/apt/sources.list.d/debian.sources` pointing at live `deb.debian.org`
  Debian and security repositories. The former Dockerfile added a snapshot
  list without disabling that inherited file. This proves an enabled live
  source; it does not identify which mirror supplied each historical package.
- **Correction:** `Dockerfile.repro` removes inherited `.list` and `.sources`
  entries, sets APT's sourceparts directory to a dedicated empty directory,
  and writes only signed, keyring-verified snapshot entries. The fixed snapshot
  is `20260223T000000Z` for `bookworm`, `bookworm-security`, and
  `bookworm-updates`. The previous `20250331T000000Z` snapshot was demonstrably
  incompatible with the newer pinned base (`gcc-12-base`, `libc6-dev`, and
  `perl-base` version conflicts), so the smallest compatible correction was to
  align the timestamp with the fixed base.
- **Regression:** Inspection inside the exact final builder reports
  `Dir::Etc::sourcelist=/etc/apt/sources.list`,
  `Dir::Etc::sourceparts=/etc/apt/event-horizon-empty-sources`, no files in
  either source-parts directory, and exactly the three snapshot lines above.
  Signature verification uses `/usr/share/keyrings/debian-archive-keyring.gpg`;
  only snapshot expiry checking is disabled. Recorded tools are GCC
  12.2.0-14+deb12u1, Python 3.11.2, Node 18.20.4, npm 9.2.0, Rust/Cargo 1.94.1,
  and `ic-wasm` 0.9.7.
- **Remaining limitation:** Debian snapshot availability remains an external
  build prerequisite; trust and TLS/signature checks were not bypassed.

## Finding E — commit-derived canonical context

- **Baseline evidence:** Confirmed by source inspection: canonical wrappers
  passed the working-tree directory to Docker. No claim is made that the
  ignored audit probe had contaminated a prior released image.
- **Correction:** `tools/scripts/lib/canonical-context.sh`, `docker-build`, and
  `verify-reproducible-artifacts` resolve HEAD once, require clean intended
  source, verify its manifest, export the exact commit with `git archive` to a
  task-owned temporary directory, verify the archived manifest, and build only
  that context. Both no-cache builds share one frozen archive. Existing valid
  release artifacts are swapped only after build and manifest success.
- **Regression:** `tools/scripts/test-canonical-context` proves an ignored
  `audit-probe.wasm` is absent and does not change context hashes; dirty tracked
  source is rejected; both reproducibility builds receive the same context and
  reported revision; and simulated build and artifact-manifest failures return
  nonzero while preserving a known-good artifact sentinel.
- **Remaining limitation:** The optional host-toolchain build remains a clearly
  non-canonical working-tree convenience.

## Finding F — bounded capacity assessment

- **Baseline evidence:** This was an assessment request, not a confirmed
  failure. No production cap or policy was inferred from the audit hypothesis.
- **Change:** Debug-only batched fixture setup and observation counters were
  added in `canisters/event-horizon/src/lib.rs`, the subscriber client, stable
  debug state, and mocks. Production polling behavior and public surfaces did
  not change for capacity policy.
- **Regression/results:** The real polling path passed:
  - global fan-out at 256 and 1,024 records: 256/256 and 1,024/1,024 one-way
    calls accepted, consuming 1,426,789,614 and 5,644,901,022 cycles;
  - specific-target fan-out at 256 and 1,024 records: 256/256 and 1,024/1,024
    calls accepted, consuming 1,542,360,764 and 6,142,410,404 cycles;
  - a 4,096-block backlog over 16 pages through the real scheduler: cursor
    reached 4,096, lane remained live, and 1,275,817,832 cycles were consumed;
  - the existing 256-target retained callback bound;
  - arbitrary `Nat` above `u64::MAX` and a 2,048-decimal-digit value, with the
    latter confirmed by the real mock subscriber.

  Stable memory was 75,563,008 bytes before and after the fan-out/backlog
  cases. Reported total memory was approximately 78.4–79.1 MB. The backlog
  cycle delta can include other scheduled lanes.
- **Remaining limitation:** Synthetic fan-out recipients prove attempted and
  accepted one-way dispatch, not execution by every subscriber. PocketIC did
  not expose a sound single instruction total across the many asynchronous
  messages, so none is reported. This finite matrix is not an unlimited-capacity
  guarantee. It exposed no repeatable trap, stuck cursor, or liveness blocker.

## Final gates and artifacts

All final gates ran from canonical artifact-source revision `875d3480…`:

- `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all`: **passed**, 92
  suites and 181 tests. This comprised 77 Rust/xtask tests (58 backend plus 19
  xtask), 17 frontend Node tests, and 87 separately discovered/executed
  PocketIC tests; PocketIC was 87 passed, 0 failed.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `./tools/scripts/security-scan`: passed. Cargo audit/OSV reported only the
  four existing filtered maintenance advisories (`backoff`, `instant`,
  `paste`, `serde_cbor`); cargo-deny passed and npm audit found zero
  vulnerabilities.
- Production export audits passed: backend application methods are exactly
  `get_instance` and `get_pricing` queries; frontend is exactly `http_request`;
  production Wasms contain no debug injection/inspection application methods.
- `npm run verify:reproducible-artifacts`: passed. Two fresh private BuildKit
  builders used the same frozen commit context and produced byte-identical
  complete outputs. Builders/caches were removed between runs.
- `./tools/scripts/docker-build` and the artifact manifest check passed.
- `DFX_IDENTITY=codex_local icp build -e local`: `Canisters built
  successfully`. The canonical Docker artifacts were restored and their
  manifest reverified afterward.

Final artifact hashes:

- backend Wasm: `f8a66d19a5215ac2f5fe283834172247e4f99cbc42aabc29d89065f5be93d1e9`;
- frontend Wasm: `02cae0b2f570d17ba4f3008d794819eb8739c054a7a65ce474a9d7f9b3b598c2`;
- `build-info.json`: `e9de53edeb070f5734447f92ae9e3a25304e850b3cadf24d58de8e9ecda7a412`;
- artifact manifest: `feb4c912cc7223948965559ad99d7b1482d1f81c2975d2a90691dab22f7c3409`;
- deterministic release archive: `ec496809da7e2b908507a6a57951ea43572568cf4a29521676d8c65e51399e1a`.

The frontend registry's backend pin is
`f8a66d19a5215ac2f5fe283834172247e4f99cbc42aabc29d89065f5be93d1e9`,
matching the canonical backend artifact.

No deploy, reinstall, upgrade, controller change/removal, ICP or cycles
transfer, CMC top-up, Jupiter payout, alias publication, or other mainnet
mutation occurred.
