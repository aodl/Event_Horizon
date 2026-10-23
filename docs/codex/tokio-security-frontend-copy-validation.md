# Tokio security and frontend callback follow-up

Starting source: `00a3a59ac4a4203ed394000c3d65ca4430b4dfd0` (clean reviewed main).
Implementation commit: `2e4bc2360eed1d57c6aa32f9dce16b2ca70f0941`.
The final evidence revision is the commit containing this report and its updated
`MANIFEST.sha256`; resolve it with `git log -1 --format=%H -- docs/codex/tokio-security-frontend-copy-validation.md`.

## Dependency remediation

Original [Dependency Security failure](https://github.com/aodl/Event_Horizon/actions/runs/39973437824):
OSV rejected Tokio 1.50.0 for `RUSTSEC-2026-0386`, an `AsyncFd` I/O-safety issue.
The live OSV API was queried during validation (advisory modified
2026-10-23T14:45:03.847864944Z); it identifies 1.51.6 as fixed.

`cargo update -p tokio --precise 1.51.6` resolved successfully. Final lockfile
changes are Tokio 1.50.0 → 1.51.6, Mio 1.1.1 → 1.2.4, Tokio macros 2.6.1 →
2.7.2, and added Syn 3.0.9 for Tokio macros, including corresponding checksums.
Unrelated Windows dependency-edge rewrites were restored and locked Cargo
metadata verified. Tokio requires Mio ^1.2.0 and Tokio macros ~2.7.0.
The workspace minimum is now 1.51.6, retaining the existing `rt` feature.
All changed/added crates declare Rust 1.71 minimums; pinned Rust 1.94.1 passed
Clippy and unit compilation. Toolchain, Docker digest, build flags, PocketIC
15.0.0, security policies, and scanner configuration are unchanged.

`cargo tree --locked -i tokio` shows both native test packages
`event-horizon-pocketic` and `event-horizon-cadence-calibration` → PocketIC
15.0.0 → Tokio, directly and through Reqwest → Hyper/H2/Hyper-util,
Hyper-rustls/Tokio-rustls, Tokio-util, and Tower/Tower-http. Neither production
package's wasm32 dependency graph includes Tokio (the combined inverse query
reports no matching Tokio package). No production dependency was added.

## Public callback copy

The Fast path paragraph now reads:

> Canonical ICP is read through its legacy block log; compatible non-ICP ledgers are read through ICRC-3. A global subscription triggers one `poke(PokeNotification)` attempt per completed poll with observed Ledger activity, setting `global_activity = true`. The independent `matches` vector contains any qualifying specific targets and may be empty.

`tools/static-check.py` rejects obsolete callback notation in public HTML and
requires the unified record, explicit global flag, independent matches, empty
vector possibility, and per-completed-poll aggregation in this paragraph. An
isolated mutation check accepted the corrected copy and rejected all five old
forms plus omission of the record, global flag, or matches detail.
A focused search of current documentation, public assets, implementation and
test comments found no other active obsolete callback descriptions. Historical
release reports remain intact. Normative full-information callbacks and optional
zero-argument Index receivers retain their existing meaning.

## Validation

Logs are retained locally under `/home/codexdev/validation-tokio/`.
Formatting and strict locked workspace/all-target Clippy passed.
`cargo run -p xtask -- test_unit` passed: 134 Rust and 138 frontend tests.
`./tools/scripts/security-scan` passed after final lockfile minimization:
Cargo audit refreshed the RustSec database (1,351 advisories), Cargo deny passed
advisories/licenses/bans/sources, npm audit reported zero vulnerabilities, and
OSV Scanner 2.3.8 reported no issues using repository `osv-scanner.toml`.
Only the four pre-existing OSV filters were applied; no advisory exceptions or
scanner exclusions were added. The new Tokio advisory was absent.

The first `test_all` run recorded 392 passes and one PocketIC client HTTP
`IncompleteMessage` failure while awaiting localhost ingress in
`external_ranges_pages_archives_neurons_and_combined_payload_bound`; it was
not a failed protocol assertion. The isolated unchanged test passed all three
256-entry variants in 91.70 seconds. A complete subsequent `test_all` rerun
passed **393 tests: 134 Rust, 138 frontend, 121 PocketIC**, with zero failures.
No assertions, production retries, or test scenarios were changed. Initial
failure and successful isolated/full rerun logs are preserved as `all.log`,
`pocketic-rerun.log`, and `all-rerun.log`. This includes old-Wasm upgrades,
permanent aliases, cross-subject/multiwatcher admission, the 1,024-watcher
workload, maximum unified notifications and arbitrary-precision amounts,
zero-argument Index interoperability, both Ledger protocols, frontend memo
validation, and financial/operational reserves.

## Canonical artifacts

`repro-source.log` records two independent uncached Docker builds from
`2e4bc2360eed1d57c6aa32f9dce16b2ca70f0941`, with byte-identical complete
artifact sets. An earlier attempt stalled after a fatal BuildKit session
healthcheck timeout and was cancelled; its logs are retained separately in
`repro-initial.log` and `buildkit-initial.log`. The successful attempt used
the unchanged canonical script, toolchain, Docker digest, and build flags.

| Artifact | SHA-256 |
|---|---|
| Backend | `2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528` |
| Frontend | `2658a8abc721c4f92f4af2c1ce3116bad08826c1b21aca6e18327515314754c3` |

Backend bytes match the reviewed baseline exactly. Frontend bytes differ from
baseline `338565e50d946d09201be87131d9c7f5319923320bf68926a17a4f6d19582067`
because the corrected HTML is embedded. Both production export audits pass
(11 backend exports with exactly three application queries; seven frontend
exports with only `http_request` as an application query). The existing
`instances.js` expected-backend pin equals the unchanged backend hash.

Closing validation repeats `npm run verify:reproducible-artifacts` and then
`./tools/scripts/docker-build` after this report and regenerated source manifest
are committed. `repro-final.log` and `docker-final.log` retain the exact printed
canonical source revision, two-build comparison, final hashes, and artifact
manifest verification. The closing response identifies that report-containing
revision and confirms the outcomes; no further evidence edits may follow those
builds without repeating them. Source-manifest verification covers this report.

## Remote security validation

GitHub inspection confirmed the original run failed its dependency security
step. Local `git push origin HEAD:main` failed because HTTPS credentials are
unavailable (`could not read Username`). The GitHub connector also rejected
Git tree creation with HTTP 403, `Resource not accessible by integration`; no
remote changes were made. The implementation commit's workflow
query returned no runs. Final-commit remote Dependency Security is **unverified**,
not passed. Publish the final revision and verify it at
[Dependency Security](https://github.com/aodl/Event_Horizon/actions/workflows/dependency-security.yml).

No deployment was performed. Production backend behavior, public Candid,
stable-memory layout, subscription economics, notification construction, and
installed-canister configuration are unchanged.
