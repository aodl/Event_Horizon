# Jupiter workflow alignment validation

Validation completed on 2026-09-29.

## Revisions and scope

- Jupiter Faucet reference revision:
  `502b2bcd88cf92279a56637f7d756a598b3b01b3`.
- Event Horizon starting revision:
  `08d167ef6130a77616d77f992d62e88e4a97e33a`.
- Workflow-alignment and test-harness revision:
  `bc47d33318e46c6e2452a886858f668b29166a31`.
- Canonical artifact source revision:
  `bc47d33318e46c6e2452a886858f668b29166a31`.
- Validation-evidence revision: recorded by the following evidence commit.
- Final provenance revision: recorded by the following manifest-only commit.

The implementation was compared directly with the named Jupiter revision.
Only shared conventions were adopted. Event Horizon did not acquire Jupiter's
local-replica, component-matrix, Playwright, custom PocketIC resolver, Nix, or
production-cutover machinery. There was no production canister source, Candid,
or embedded frontend asset change.

## Workflow result

The public xtask surface is now `frontend_setup`, `test_unit`,
`test_pocketic_integration`, and `test_all`. The old `unit`, `check`,
`pocketic`, `test-all`, `security`, `release`, `canonical`, `repro`, and
`validate` commands were removed. Security, reproducibility, and canonical
release builds remain script/npm concerns.

`frontend_setup` fingerprints the lockfile and relevant npm/Node versions. It
runs `npm ci` when dependencies are absent or stale and avoids reinstalling an
already-consistent locked dependency tree. Managed test entry points therefore
do not fail merely because `npm ci` was omitted. Frontend unit tests use normal
Node isolation:

```text
node --test canisters/frontend/test/*.test.mjs
```

Repository validation no longer imports `tomllib` or `tomli`; Python 3.10-class
hosts use standard-library code plus locked `cargo metadata`.

## Consolidated test runner

The Event Horizon runner captures and displays one coherent combined
stdout/stderr stream on Unix, retains the complete captured output, parses
Cargo/libtest and Node summaries, rejects successful zero-test selections, and
prints all suite results before returning failure. Rust failure blocks retain
the test name, assertion/panic body (including `left` and `right` when libtest
provides them), and a precise rerun command in the final `Failures:` section.
Independent later suites still run after a suite failure.

Commands have a three-hour default bound. A positive integer override is
accepted through `EVENT_HORIZON_TEST_COMMAND_TIMEOUT_SECS`; invalid values are
rejected before spawn. On Unix every command owns a process group. Timeout or
capture failure kills that group, direct-child exit while a descendant retains
the output stream is detected, and post-failure draining is bounded.

All 12 runner/xtask unit tests passed. They cover Rust success and failure
summaries, multiple failures, assertion preservation, Node summaries, failed
children, final-summary status, continuation to later suites, zero-test
rejection, invalid timeout rejection before spawn, a real timeout with
descendant cleanup, descendant-held output, and ordered combined failure
output.

## SNS neuron-range failure investigation

The exact failure text supplied from the original complete run was:

```text
failures:
    tests::sns_neuron_range_uses_verified_governance_owner

test result: FAILED.
71 passed; 1 failed
```

The supplied output did not contain the panic body or `left`/`right` values, so
none are inferred here. A later attempt to reproduce the starting revision in
a detached temporary worktree could not reach the assertion because the
duplicate compilation exhausted the host root filesystem; that unrelated
linker failure is not treated as test evidence.

Inspection and targeted assertions ruled out an owner, admission, Candid, and
cross-test-state defect. Before sending transfers, the repaired test proves
that the discovered SNS profile's verified Governance principal is the mock
Governance canister, both observed/admission cursors are bootstrapped, and all
three `0..=2` subscriptions exist. It then observes exactly the expected
`10`, `20`, and `30` matches; the deliberate `999` transfer to the wrong owner
does not match.

The defect was the test harness's assumption that five unconditional
PocketIC ticks always deliver an outbound one-way callback. One-way delivery is
best effort and scheduler-round sensitive. Production `poke` remains one-way
and has no added retry or wait. The harness now ticks and queries until the
expected subscriber condition becomes visible, subject to a 20-tick
deterministic ceiling. Exhaustion reports the expected condition, ticks and
wall time consumed, last subscriber observation, and Event Horizon state. The
same helper replaced fixed positive callback waits in the other affected tests.

The formerly failing isolated test passed 10 consecutive runs. The complete
serial PocketIC suite subsequently passed twice through managed entry points:
72 passed, 0 failed each time.

## Documentation and CI

Live documentation is grouped under `docs/architecture/`,
`docs/development/`, `docs/operations/`, and `docs/security/`, while the
subscriber guide and historical provenance retain their Event Horizon-specific
locations. Backend and frontend component READMEs describe their respective
surfaces and link to the normative protocol and operating material.

The testing guide retains the detailed PocketIC coverage catalogue. The
reproducible-build guide documents all four canonical outputs, export and
method-surface audits, uncompressed Wasm, manifest and mainnet module-hash
verification, retained evidence, and controller-removal re-verification. The
frontend README retains the certified/static asset model, static registry,
`@icp-sdk/core`, `https://icp-api.io`, embedded mainnet root key, declaration
modes, full-width integers, memo limit, pricing semantics, and authoritative
backend admission. The dependency-security guide reflects the repository's
actual security script and policy files.

`Behavioral Tests` is manual (`workflow_dispatch`) and runs `test_unit`.
`Dependency Security` runs on pull requests and pushes to `main` or `master`.
Both use pinned action SHAs, derive Rust from `rust-toolchain.toml`, install the
required components/target, and explicitly select Node 24.15.0.

## Validation results

Validation of the source/workflow contents that became
`bc47d33318e46c6e2452a886858f668b29166a31` produced:

- `cargo test --locked -p xtask`: 12 passed, 0 failed.
- `cargo run -p xtask -- test_unit`: 4 suites passed, 79 behavioural tests
  passed (68 Rust and 11 frontend), 0 failed; 72 intentionally ignored
  PocketIC tests were not selected.
- `cargo run -p xtask -- test_pocketic_integration`: 72 passed, 0 failed.
- `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all`: 5 suites passed,
  151 behavioural tests passed (68 Rust, 11 frontend, 72 PocketIC), 0 failed.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `./tools/scripts/security-scan`: passed. The four pre-existing filtered
  Rust maintenance advisories remained documented; cargo-deny passed, npm
  audit reported zero vulnerabilities, and OSV reported no unfiltered issue.
- `npm run verify:reproducible-artifacts`: two no-cache Docker builds were
  byte-identical. The exact clean source-commit rerun also passed after pruning
  only unused BuildKit cache that had filled Docker's storage filesystem.
- `./tools/scripts/docker-build`: passed and populated the canonical artifact
  set from the clean source commit.
- `(cd release-artifacts && sha256sum -c release-artifacts.sha256)`: all three
  manifest entries passed.
- `DFX_IDENTITY=codex_local icp build -e local`: passed. The canonical Docker
  build and manifest verification were rerun afterward.
- Artifact-safety regression: forced failing `build-release` and
  `docker-build` attempts both preserved a previously valid
  `release-artifacts/` directory byte for byte.
- Source-manifest generation and verification: passed before the source commit.

## Canonical artifacts

- Backend `event_horizon.wasm` SHA-256:
  `91a12fe2f63edb5f7a3bab311a34294195a45bdde8df6eae3f83d3694387ad77`.
- Frontend `event_horizon_frontend.wasm` SHA-256:
  `9028449c0caccd1358dd29b5055484d0e5fef9e3a498fe4a11fb2289b0a26187`.
- Both executable hashes are unchanged from the prior reviewed production
  bytes.
- `build-info.json` SHA-256:
  `e9ca78f449d6788a6e5629cf84385596d3086e49e8f6f6a28943d1bbf36cf073`.
- Deterministic canonical release archive SHA-256:
  `d1262e25bc352f9a32b85f1c91a29d4f4c2ba93a330eb3b96022958f5638ee0a`.

The release archive hash is reproducible with:

```bash
tar --sort=name --mtime='@1700000000' --owner=0 --group=0 \
  --numeric-owner --format=ustar -C release-artifacts -cf - \
  build-info.json event_horizon.wasm event_horizon_frontend.wasm \
  release-artifacts.sha256 | gzip -n | sha256sum
```

The executable inputs remained unchanged; `build-info.json` changed only
because the artifact-safe `tools/scripts/build-release` implementation is one
of its recorded provenance inputs.

## Safety

No deploy, reinstall, upgrade, controller change, transfer, cycle top-up,
alias publication, or other mainnet mutation occurred.
