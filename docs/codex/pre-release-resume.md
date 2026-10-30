# Interrupted pre-release closure recovery

Recovered 2026-10-29 at HEAD dd4a88993b92f522a08296786e9afd453b4a3101, branch codex/cycle-optimisation-81c5a0d. All four intentional staged files retained; no unstaged changes at recovery; cached whitespace check passed. No validation process was running.

Both ignored historical fixtures exist and their actual SHA-256 values match staged provenance (production e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca; debug 43fb4206372a42c1ef30238101fdc41d7ef44ccc22945461c6eed28e56bfef7c). Canonical backend f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970 and frontend 210d4de6587a19228801f1d97264cb87d0c8d1117632ecb6f7051829b3b441c8 match expected bytes. Fixtures were ignored, not tracked.

Historical receipts are under observations/release-closure and live observations under observations/20261028T213120Z. Their existence alone is not test success; inspect statuses before reuse. New test validation remains to be established. No mainnet mutation authorised.

## Release findings

Starting commit: `dd4a88993b92f522a08296786e9afd453b4a3101` on
`codex/cycle-optimisation-81c5a0d`. The final closure commit is the commit
containing this report: `git log -1 --format=%H -- docs/codex/pre-release-resume.md`.
Its exact SHA and publication outcome are in the final operator handoff. Existing [PR #12](https://github.com/aodl/Event_Horizon/pull/12)
is the sole release PR. No mainnet mutation, funding, controller change or merge
was performed.

**Recommendation: ordinary state-preserving backend upgrade, once release
blockers are cleared.** No reason to erase state has been established. The
existing installation already uses the 10/20/1,000 pricing regime. Earlier
pricing-reset reinstall instructions were obsolete. Having no users or
subscriptions does not justify discarding financial journals, reader cursors,
pricing history or frozen prices.

### Installed-version evidence

The historical anonymous public read at 2026-10-28T21:31:20Z and fresh public
reads through 2026-10-29T10:58:56Z report backend
`eo6ei-gaaaa-aaaar-qchra-cai` at module hash
`e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca`.
[Final live reads](pre-release-data/final-live-reads.json) and
[retained observations](pre-release-data/) include original timestamps, revision,
CLI outputs, command exit results and SDK query outputs. The source-to-module
identification is a byte-hash match against the reproduced historical module,
not a claim that source inspection alone identifies live code. Public backend
Candid metadata was unavailable in the previous read-state attempt; the
historical Candid and parsed binary exports are separately retained from source
and the exact hashed fixture.

The separately installed frontend still reports
`9a0f1978e0853a099433f8b1ea2f056f854443f76ed3c3cf870066df8486a7ef`,
which differs from the canonical candidate below; it has not been upgraded here.

Fresh instance reads confirm canonical ICP Ledger, no SNS Root and no surplus
recipient. Pricing is initialized at account 10/range 20/global 1,000 ICP, with
next prices 10/20/991 and retained CMC observations. The public status query at
10:59:04Z reports running, 4,379,567,985,571 cycles and 69,942,772 bytes memory;
these are timestamped readings, not current guarantees. The later ICP Ledger
balance query returned **0 e8s** for the backend's default account (query exit 0).
No transfers were sent.

Public observations do not expose subscriptions, reader cursors or the complete
financial journal. The no-users/no-subscriptions pre-release assumption is
operator context, not established by an empty log. An absent retained log or
zero ICP account balance does not prove no pending CMC notification, earlier
transfer obligation or recovery identity. Preserve such unknown state by upgrade.
Reinstall would erase stable configuration/profile, subscriptions, cursors,
pricing observations/freezes, CMC/surplus journal and policy; external ICP
holdings and canister cycles are separate and would not themselves be erased.
No desirable reset or financial-state loss has been approved.

### Historical fixture and upgrade boundary

Source: `54d8b4ba99e6392cfbfa3a5d064ce8f57f8378d7`. Both verified binaries are
explicitly tracked despite the general Wasm ignore rule. Fresh checkouts contain
both. [Provenance](../../tests/fixtures/event_horizon_54d8b4b.provenance.txt) records
locked dependency/configuration hashes and the deterministic
[rebuild script](../../tests/fixtures/rebuild-54d8b4b.sh): Rust 1.94.1, both packages,
release z/LTO/one codegen unit/panic abort, remapped paths, fixed epoch and
ic-wasm 0.9.7 shrink.

| Artifact | SHA-256 |
| --- | --- |
| Historical production | `e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca` |
| Historical debug companion | `43fb4206372a42c1ef30238101fdc41d7ef44ccc22945461c6eed28e56bfef7c` |
| Optimised canonical production | `f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970` |

The new discovered PocketIC scenario is
`observed_54d8b4b_production_upgrades_preserving_financial_state`. Its four cases
cover pending CMC transfer and notify with surplus disabled/enabled. Historical
debug code creates genuine historical journal bytes through Ledger/CMC mocks;
it does not serialize a current host FundingState into old storage. Historical
funding, surplus, pricing, instance and config implementation is unchanged in
the candidate. The source defines stable IDs 0–8; its production binary has the
two historical application queries, no debug/application updates, and the CDK's
internal timer executor. Current preview is additive; new regions 9–11 do not
rewrite the historical stores.

The test compares every effective historical stable region across the exact
old production and canonical candidate modules, including financial/pricing
bytes, nonzero Ledger cursors, self subscriptions and globals. The only expected
metadata transition is production recomputing polling mode from actual liquidity.
The historical reader first consumes the block created by its seeded CMC
transfer; failing to settle that fixture block had correctly produced catch-up,
not cursor loss. For enabled surplus, historical funding seeds a legitimate
150 T epoch low-water mark below the fixture's current 200 T balance. A minimum
seeded at today's exact balance would correctly fall by startup execution costs,
even on old production; the funding/policy implementation is unchanged. This
keeps exact region-8 byte assertions valid rather than omitting that state.
Redundant inspection upgrades were removed after PocketIC rate-limited
repeated installs. The candidate now stays in production through notification
and timer-driven recovery before a final debug inspection. No financial or cursor assertion was dropped.
It then checks initialized alias state/new external-store emptiness, exact modern
subject-qualified notification from production, and production financial recovery.
Debug inspection remains outside the production module; the obsolete callback
ABI is not retained in production.

The isolated scenario **passed**: four cases, one test, zero failures,
zero ignored (128 other scenarios filtered), exit 0. The
[receipt](pre-release-data/historical-isolated-result.json) records the exact test
source and log digests. The normal `test_all` run also discovered and executed this scenario with
one pass/zero failures/zero ignored, as retained in its
[final suite transcript](pre-release-data/final-historical-suite-result.txt).
The complete frozen-source combined gate also passed, as reported below. Earlier diagnostic failures and their logs remain under
`observations/pre-release-resume/`.

### Jupiter installed-version matrix

Read-only status/Candid/alias calls were repeated on 2026-10-29; final module
hashes and the missing-alias rejection were reconfirmed around 10:59Z. Tracked
Candid transcripts trim trailing blank output lines, and copied command listings
trim trailing spaces; raw CLI bytes remain in
`observations/pre-release-resume/jupiter/`. Current Jupiter
source was inspected at `e60b79fc428b63cde629174a0f1c40d5fa372623` (clean tree).
Its memo-policy publishes D → Disburser and E → Event Horizon; only E. receives
the special Event Horizon payment handling. D is an identity, not a payment route.
Its four receivers use unified `poke(PokeNotification)`, authenticate Event
Horizon and filter relevant subjects/targets/amounts. Timers and authoritative
reconciliation remain independent. These are source facts; they do not prove
installation.

| Canister | Observed live raw module hash | Installed prerequisite result |
| --- | --- | --- |
| Faucet `acjuz-liaaa-aaaar-qb4qq-cai` | `772a4da72c296eb9296fc95b70bd056475efd65416f795eaab5f181f1926410a` | Published Candid still vector poke; unified ABI blocked |
| Disburser `uccpi-cqaaa-aaaar-qby3q-cai` | `f7568b3d4fd4a3a2109e6f16cb486b4da1e31987ebf34775ce6f2b856dab7862` | Published Candid still vector poke; unified ABI blocked |
| Historian `j5gs6-uiaaa-aaaar-qb5cq-cai` | `fa4911b98b5e0fedccea0a57d62be0a0b1696334f3cb0fe440d41917fdfd584a` | Vector poke; real alias query rejects IC0536 missing method |
| Relay `u2qkp-aqaaa-aaaar-qb7ea-cai` | `b603668def7304c0a669c019f3f9cd024fa37b03004c852e11c11744e278e358` | Published Candid still vector poke; unified ABI blocked |

Retained local Jupiter raw artifact hashes (Faucet `1952ddcf…d5d9`, Disburser
`df4dda84…832a`, Historian `716b75ec…98c7`, Relay `8db17b83…b661`) differ from all
live hashes. Their source/build provenance was not independently re-established
by this Event Horizon task, so they are comparison evidence, not new Jupiter
deployment approvals. Full values and local build metadata are retained in the
[comparison receipt](pre-release-data/jupiter-local-artifact-comparison.json). Live D/E publication, E. routing, receiver authorization/filtering
and independent reconciliation of the intended new release remain **unverified**.
No update call to a receiver and no funding transfer was used to test mainnet.

Jupiter operator ordering, from its `docs/operations/deployment.md` and
`docs/relay-setup-recovery.md` at the inspected revision:

1. Complete Jupiter's own artifact/security/preflight review and inventory.
   Faucet requires an empty active-payout journal; Disburser requires its
   maturity/payout quiescence. These boundaries were not established by public
   reads here and require its operator's separate decision.
2. Pause/drain Historian by stopping it and waiting for `Stopped`; upgrade it
   **in place**, with no arguments for a routine configuration-preserving
   upgrade. If its frontend Candid changes, deploy the matching Jupiter frontend
   while Historian remains stopped, then start and verify historical counts,
   mappings, exact-route evidence, D/E aliases and hashes. Never reinstall it.
3. Upgrade Faucet and Disburser at their documented quiescent boundaries, using
   routine no-config upgrade arguments (none), and verify their receiver hashes.
4. Treat canonical Relay separately: its reviewed full replacement `InitArgs`
   are required even for upgrade; operational heap resets and the first ordinary
   allocation is `BaselineOnly`. Inventory active stable splitter/reward work;
   do not erase its independent journals. If the chosen release changes reward
   context, satisfy Jupiter's SNS Rewards snapshot/context prerequisites first.
   Updating Historian's embedded Relay changes **new children only**; existing
   controllerless self-service Relays are immutable and cannot be upgraded.
5. Only after these installed receiver/alias prerequisites are verified, upgrade
   Event Horizon's sender, then its own frontend. Reconciliation timers must
   remain sufficient throughout a window of missed or rejected hints.

Existing isolated suites already exercise sender notification content, exact
route admission, prospective-cursor admission, alias refresh and receiver
filtering/reconciliation. The historical installed-module boundary was the
missing Event Horizon test. Targeted Jupiter receiver/alias/memo/wire tests are
run locally as independent suites. A new production cross-repository dependency
or combined framework would not remove the installed-version blocker and is
not justified for this closure.

### Observation tooling and deployment sequence

Installed CLI 1.2.0 help confirms public status, anonymous identity, query and
checked-in Candid flags. Application calls use `--query --identity anonymous`
and the repository DID. Public status uses `--public`. Logs use the CLI public
log read. Native management status/metrics queries use an anonymous pinned SDK
agent with explicit effective routing, because CLI 1.2.0 has no such call flag.
The helper allowlists only two read methods and does not fetch a new root key.
Metrics correctly return **IC0512 controller-only rejection** for anonymous
reads; this is retained as failure, never zero usage. The script captures all
other reads, records each exit status plus shell-quoted command, uses unique
timestamped output directories, and returns nonzero if any read fails. An
injected failed pricing read verifies that later observations remain intact.

The [deployment runbook](../operations/deployment.md) is the single current
procedure. Backend upgrade, exceptional destructive reinstall, frontend upgrade,
new instance install, and later controller removal are separate decisions.
Obsolete active pricing-reset instructions and retired X routing were removed;
production query lists now include `preview_subscription`.

Operator-only activation checklist (none performed here):

1. Verify security and behavioural CI at the exact release head. Review the
   source manifest and canonical artifacts.
2. Reobserve mappings, installed hashes/configuration, cycles/ICP, pricing and
   operational evidence. Stop on a changed backend hash until its upgrade
   compatibility is reviewed; retain unknown financial obligations.
3. Under separate Jupiter authorization, upgrade required receivers and
   Historian alias publication first, following Jupiter's reviewed Relay
   lifecycle/dependency plan. Verify resulting hashes, D/E bindings, E. route,
   trusted caller, subject filters and independent timers before relying on hints.
4. Under separate Event Horizon authorization, upgrade the existing backend
   argument-free with the canonical Wasm. Preserve stable state; verify
   hash/configuration, pricing, alias refresh, financial progress and callbacks.
5. Separately upgrade the existing frontend with the matching backend pin;
   verify certified assets, instance/pricing/preview queries and memo builder.
6. For new instances or any separately approved reinstall, establish the
   prospective cursor/profile/pricing baseline before new endowment activity.
   Do not expect old payouts to replay. Funding is a distinct operator action.
7. Observe while controlled. Only after the separate observation period and
   controller-removal checklist, consider irreversible backend controller removal.

### Validation, artifacts and release blockers

The frozen-source combined gate exits **0**: **146 Rust + 138 frontend +
129 PocketIC = 413 passed**. Discovery includes the added historical scenario,
which executes with zero ignored tests. Final fmt, strict locked all-target/
all-feature Clippy, standalone test_unit, observation-tool tests, static and
canonical-context checks, both production export audits, and source-manifest
checks pass. Jupiter's independent boundary suites pass 28 native + 7 PocketIC
checks; its source tree is unchanged.

[Command/exit/log receipts](pre-release-data/validation-results.json) and the
[final test summary](pre-release-data/final-test-summary.txt) record these results.
Raw final runs are `observations/pre-release-resume/final-checks.json` and
`final-all-status.json`. The first combined attempt had 129 PocketIC passes but
returned failure because its earlier unit phase compiled a temporary test-only
hex reference. That failure is retained and was superseded by the clean final
unit and combined gates; it is not counted as a passing combined run.

Canonical backend remains `f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`;
frontend remains `210d4de6587a19228801f1d97264cb87d0c8d1117632ecb6f7051829b3b441c8`.
[Artifact identity receipt](pre-release-data/canonical-identity.json) verifies
all recorded build-input hashes and an empty production source/build-recipe
diff from dd4a889. The retained two-independent-uncached-build receipt/log hash
was rechecked, as were final canonical Wasm bytes and all artifact-manifest
entries. No production input changed; a new two-build campaign is unnecessary.
Historical/debug fixtures never substitute for these release artifacts.

On 2026-10-30 the unmodified security scan at `9df3f162` still exited 1:
audit/deny/npm passed and OSV 2.3.8 reported only locked block-buffer 0.10.4 for
GHSA-qwgh-2vcv-g2f7. GitHub still published `<0.12.1`, OSV still imported
introduced `0`, and [correction PR #10755](https://github.com/github/advisory-database/pull/10755)
was open and unmerged. Current PR-head Dependency Security run
[40922693665](https://github.com/aodl/Event_Horizon/actions/runs/40922693665)
therefore failed only that medium finding after the other three scanners passed.

A temporary exact-version resolution was merged in PR #12. Before OSV applied
the single GHSA filter, a dependency-free guard parsed the actual root lockfile
and required exactly one `block-buffer` package at 0.10.4 from the crates.io
registry source with checksum
`3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71`.
Synthetic negative and shell-ordering tests prevent that advisory-wide filter
from silently broadening. The four existing filters and all scanners remained.
The canonical local gate exited 0 **under this guarded exception**: cargo-audit
0.22.2 passed with the same four allowed unmaintained warnings, cargo-deny 0.19.8
passed all four policy groups, npm 11.12.1 reported zero vulnerabilities, and
OSV Scanner 2.3.8 reported no unfiltered issues after the exact guard and its 16
tests passed. That was not an exception-free security result.

Post-merge on 2026-10-30, correction PR #10755 merged and live OSV metadata
propagated the maintainer's `>=0.11.0,<0.12.1` affected interval. The follow-up
cleanup removed the GHSA filter and otherwise-unused guard while preserving the
four unrelated filters. Current security results are therefore produced by the
standard scan without this exception. Jupiter installed compatibility remains
an independent activation blocker.
