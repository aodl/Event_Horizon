# Existing PR handoff

Status: [PR #12](https://github.com/aodl/Event_Horizon/pull/12) already exists
and remains open. Do not create another PR. Its last confirmed remote head is
dd4a88993b92f522a08296786e9afd453b4a3101; publish the resumed closure commit on
the same branch and verify CI for that exact head. The current release record
and activation checklist are in [pre-release resume](pre-release-resume.md).

- Repository: aodl/Event_Horizon
- Head: codex/cycle-optimisation-81c5a0d
- Base: main
- Immutable validated recipe: d13d1138e2fde4af18bd0d3f51f777470037f93f
- State: open; security clearance remains blocked; no merge is authorised
- [Existing PR](https://github.com/aodl/Event_Horizon/pull/12)

Title: **Reduce Event Horizon cycle costs and select reproducible hybrid release profiles**

## Body

Reduce sender work with immutable contexts, borrowed matching/extraction,
scan-owned accumulation, page-local record highs, lazy globals, shared encoding
and unchanged-write suppression. Preserve PokeNotification, stable IDs0–11,
admission and dispatch order, exact-route evidence, financial recovery and cadence.

Canonical ICP sender reductions: ordinary 256: 25.13%, new256range 45.05%,
overflow 4096: 40.61%; generic 9.88%,39.03%,21.40%. Backend SHA remains
f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970, preserving
72 canonical comparison pairs, complete stable-state and baseline-upgrade evidence.
No unconditional daily-burn saving is claimed.

Select backend 2/frontend full-workspace z, preserving feature unification with the
same package set in isolated outputs. Frontend size4009935→3757901; installation/
certification13914479233→12460593460cycles; reported idle/day1176063315→1169624340;
heap unchanged. Twenty certified responses pass per option; all 29 public assets
also match completely. Query statistics cover13/20 calls and are inconclusive.
Backend pin is unchanged; frontend SHA is
210d4de6587a19228801f1d97264cb87d0c8d1117632ecb6f7051829b3b441c8.

Validation at d13d113: fmt, strict all-feature Clippy,146 Rust + 138 frontend+
128PocketIC=412tests, static/source manifest, Candid and production export audits
pass. Two independent uncached canonical Docker builds compare all generated
files successfully; populated artifacts have the matching Wasm identities and
verified artifact manifest. Final evidence-only outputs are checked separately.

Security is NOT cleared. Audit/deny/npm pass; OSV fails
GHSA-qwgh-2vcv-g2f7/block-buffer 0.10.4. Maintainer range starts0.11.0; imported
metadata starts0. Independent source review and191 native panic cases support a
correction. A complete correction request and separate unapproved exact-version
exception proposal are tracked. [Advisory correction PR #10755](https://github.com/github/advisory-database/pull/10755) was submitted and remains open; no exception was activated.

See docs/codex/pre-release-resume.md for the historical installed-version upgrade test, updated runbooks, Jupiter installed-version blockers and final closure receipts. The older optimisation evidence remains in docs/codex/cycle-optimisation-release-closure.md.
Required before release: authoritative metadata/policy resolution and actual
passing Dependency Security on this PR's final head. Record the run URL, headSHA
and conclusion after it finishes; no future CI success is claimed here.

No production Rust/Candid/stable-layout change in closure. No merge, deployment,
mainnet upgrade, controller action or real transfer is authorised.
