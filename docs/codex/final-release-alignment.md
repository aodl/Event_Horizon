# Final release documentation alignment — 2026-10-31

Base: `origin/main` verified by fetch and remote ref at
`614ed4e89570e1459870a06dadd01dab23a612c7`, after merged PRs #12, #13 and #14.
Branch: `codex/final-release-documentation-alignment`. The reviewed documentation
commit is the commit containing this report; resolve it with
`git log -1 --format=%H -- docs/codex/final-release-alignment.md`.

## Deployment boundary

The [deployment runbook](../operations/deployment.md) permits the ordinary,
argument-free, state-preserving backend upgrade, then a separately authorised
frontend upgrade and unsubscribed observation, independently of Jupiter's
schedule. Canonical-artifact mode, `--no-create`, hash/configuration checks,
financial-state preservation and certification checks remain required.

Funded subscription activation remains deferred pending installed alias
publication, canonical identity resolution, Faucet routing, exact Historian
admission evidence, compatible receivers and an adequate observed cadence/cycle
reserve. Historical funding is not assumed to replay. No callback evidence is
expected without an admitted subscriber. Backend controller removal retains
[its separate, stricter irreversible gate](../operations/controller-removal.md).
The [pre-release report](pre-release-resume.md) retains its original timestamped
observations and test results, with a dated clarification and revised checklist.

## Code and existing behavioural evidence

- [Alias refresh](../../canisters/event-horizon/src/aliases.rs) has its own timer,
  one attempted refresh per UTC day and a bounded Historian call. Failed refresh
  logs `ALIASES_UNAVAILABLE` and does not replace the accepted cache. Startup
  schedules [polling and aliases independently](../../canisters/event-horizon/src/lib.rs).
- [Polling](../../canisters/event-horizon/src/polling.rs) resolves aliases during
  admission and requires successful exact-route evidence from Historian. Failure
  does not admit the declaration or create a recovery retry; ordinary matching
  and dispatch use stored admitted principals. Ledger/profile failures and
  reserve protection remain separate constraints; external failures are not
  universally harmless.
- Existing [PocketIC scenarios](../../tests/pocketic/src/lib.rs)
  `aliases_paging_failures_daily_cache_and_upgrade` assert cursor advancement
  through malformed/rejected alias snapshots and cache preservation across
  upgrade. `historian_outage_does_not_halt_observation_or_create_admission_retry`
  verifies an already-admitted subscriber's callback during outage, failed new
  admission, no replay after recovery and admission after a later qualifying payout.
  This evidence supports the documented deployment boundary; no new Rust or
  PocketIC test is needed for this documentation-only change.

## Exact post-merge mainline CI

GitHub Actions API run and job records were inspected on 2026-10-31. Both runs
tested **`614ed4e89570e1459870a06dadd01dab23a612c7`** on branch `main`:

| Workflow | Definition | Event | Run | Final status | Final conclusion |
| --- | --- | --- | --- | --- | --- |
| Dependency Security | `.github/workflows/dependency-security.yml` | `push` | [41206338057](https://github.com/aodl/Event_Horizon/actions/runs/41206338057) | `completed` | `success` |
| Behavioral Tests | `.github/workflows/behavioral-tests.yml` | `push` | [41206338073](https://github.com/aodl/Event_Horizon/actions/runs/41206338073) | `completed` | `success` |

The [security gate](../../tools/scripts/security-scan) runs audit, deny, npm audit
and OSV scanning of **both** `Cargo.lock` and `package-lock.json`. The temporary
`GHSA-qwgh-2vcv-g2f7` exclusion and dedicated guard are absent. The four other
[OSV exclusions](../../osv-scanner.toml) are unchanged from pre-cleanup mainline
`12268955dca440fef5f7272221009d43c8df3a12`: `RUSTSEC-2025-0012`,
`RUSTSEC-2024-0384`, `RUSTSEC-2024-0436`, `RUSTSEC-2021-0127`.
The [behavioural workflow](../../.github/workflows/behavioral-tests.yml) runs
automatically on pull requests and pushes to `main`, and supports manual dispatch.
Neither workflow nor security policy is changed by this PR.

## Production identity and validation

Production inputs and canonical recipes are unchanged from the base and from
the retained [canonical identity evidence](pre-release-data/canonical-identity.json)
at `dd4a88993b92f522a08296786e9afd453b4a3101`. Comparison includes backend/frontend
sources and public assets, Candid, Cargo manifests/lock, Node manifests/lock,
toolchain, `SPEC.md`, `icp.yaml`, `Dockerfile.repro`, build scripts, canonical-context
helpers and the Wasm auditor. Expected uncompressed canonical identities remain:

| Artifact | SHA-256 |
| --- | --- |
| Backend | `f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970` |
| Frontend | `210d4de6587a19228801f1d97264cb87d0c8d1117632ecb6f7051829b3b441c8` |

Retained canonical Wasm, build-info and artifact-manifest hashes were rechecked
against the identity receipt, and recorded build-info inputs match current source.
Both Wasm export audits pass with the unchanged auditor. The
[retained independent two-build receipt](release-closure-data/status.json)
remains applicable. A new Git revision and regenerated
source manifest record documentation changes, not changed production Wasm bytes.
No new canonical build campaign or full PocketIC suite is claimed. The previous
**146 Rust + 138 frontend + 129 PocketIC = 413** pass remains historical evidence
in the [pre-release report](pre-release-resume.md#validation-artifacts-and-release-blockers).

Validation completed with exit 0: `cargo fmt --all -- --check`, source-manifest
generation and membership/hash verification, `cargo run -p xtask -- test_unit`,
all 135 local documentation links/fragments and `git diff --check`. The unit
matrix passed all five suites: repository static validation, manifest,
canonical-context fixtures, **146 Rust + 138 frontend = 284 tests**, zero failures.
Its 129 ignored PocketIC scenarios were not executed by the unit target.
Manifest consistency (474 entries) and the same links are also verified from a
fresh Git archive of the final commit. Full Clippy and reproducibility retain
the previous evidence because their inputs are unchanged; final live
deployment/finalisation checks remain operator duties.

No mainnet mutation, subscription, ICP transfer, controller change or Jupiter
change was performed. External installed compatibility and live activation
verification remain open operator gates; they do not block unsubscribed deployment.
