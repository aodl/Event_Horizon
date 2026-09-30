# PocketIC process-isolation validation

Validation completed on 2026-09-30.

## Revisions and scope

- Starting revision:
  `e1b218577f575d46ff86b072ddd8eae2f6c9b0f8`.
- PocketIC runner and documentation revision:
  `77d194bf85558336908dab3dbc5758eea3d5d4be`.
- Validation-evidence revision: recorded by the following evidence commit.
- Final provenance revision: recorded by the following manifest-only commit.

This pass changes xtask orchestration, runner tests, documentation, and source
provenance only. Production canister code, Candid, frontend runtime assets,
callback behavior, and deterministic tick bounds are unchanged.

## Failure pattern and execution model

Two complete monolithic ignored-test runs had failed late with the same
PocketIC HTTP transport failure at different server instances:

```text
hyper::Error(IncompleteMessage)
... /instances/67/update/await_ingress_message
```

and:

```text
hyper::Error(IncompleteMessage)
... /instances/64/update/await_ingress_message
```

The affected tests were
`staged_distinct_admission_is_not_durable_before_admission_cursor_commit` and
`sns_profile_failures_log_once_per_episode_and_recover`. Both passed
independently, and neither failure was an application assertion. The previously
investigated `sns_neuron_range_uses_verified_governance_owner` also continued
to pass independently.

`test_pocketic_integration` no longer puts all ignored tests in one Cargo test
process. It obtains the ignored library-test listing from libtest, parses only
valid `: test` entries in discovery order, rejects an empty discovery, and runs
each exact name serially in a fresh Cargo test process and PocketIC lifecycle.
Every exact invocation receives `POCKET_IC_MUTE_SERVER=1` and
`RUST_TEST_THREADS=1`. There is no failure retry.

Each exact invocation retains the existing combined output capture, bounded
timeout, process-group cleanup, descendant-pipe detection, libtest summary and
zero-test validation, panic/assertion extraction, and final failure reporting.
Later discovered tests continue after a failure. The final summary records one
outcome per name and prints the discovered aggregate. A known failing name is
rerun with the exact `--lib`, quoted filter, `--exact`, `--ignored`,
`--nocapture`, and single-thread arguments.

## Runner tests

All 18 xtask unit tests passed. New coverage verifies:

- ignored-test listing parsing while ignoring cargo, warning, benchmark, and
  other non-test lines;
- deterministic discovery order;
- rejection of zero discovered tests;
- exact Cargo argument and environment construction;
- continuation after one failed exact test;
- aggregate counts across exact outcomes; and
- printing the exact isolated rerun command.

The prior timeout, process cleanup, combined-stream, parser, assertion-detail,
suite-continuation, and zero-behavioural-test tests remain green.

## Validation results

- `cargo test --locked -p xtask`: 18 passed, 0 failed.
- `cargo run -p xtask -- test_pocketic_integration`: discovered 72 tests;
  72 passed, 0 failed in 72 independent exact invocations.
- `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all`: 76 outcomes and
  157 tests passed, 0 failed: 74 ordinary Rust tests, 11 frontend tests, and
  72 isolated PocketIC tests. The PocketIC aggregate was 72 passed, 0 failed.
- Both previously transport-affected tests passed in both managed isolated
  runs. The SNS neuron-range test also passed in both runs.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `./tools/scripts/security-scan`: passed. The same four documented
  test/tooling or maintenance advisories were filtered; cargo-deny passed, npm
  audit reported zero vulnerabilities, and OSV reported no unfiltered issue.
- Source-manifest generation and verification: passed for the runner/docs
  source commit.

## Production artifacts

The canonical artifact manifest verifies. Production executable hashes remain
unchanged:

- backend `event_horizon.wasm` SHA-256:
  `91a12fe2f63edb5f7a3bab311a34294195a45bdde8df6eae3f83d3694387ad77`;
- frontend `event_horizon_frontend.wasm` SHA-256:
  `9028449c0caccd1358dd29b5055484d0e5fef9e3a498fe4a11fb2289b0a26187`.

## Deployment-document clarification

An Event Horizon backend may be installed, initialize its immutable
observed-ledger profile, and observe its configured ledger without a Jupiter
Faucet alias. A reviewed alias is required before the instance is opened for
subscriber use because Faucet payout plus matching Historian route evidence is
the declaration funding and admission mechanism. No alias or deployment state
was changed by this work.

## Safety

No deploy, reinstall, upgrade, controller change, transfer, cycle top-up,
alias publication, or other mainnet mutation occurred.
