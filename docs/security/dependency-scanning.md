# Dependency scanning

The canonical dependency-security gate is:

```bash
./tools/scripts/security-scan
```

It requires `cargo-audit`, `cargo-deny`, npm, and `osv-scanner`, then runs:

```bash
cargo audit
cargo deny check advisories licenses bans sources
npm audit --omit=dev
osv-scanner scan -L Cargo.lock -L package-lock.json
```

`deny.toml` enforces the repository's advisory, license, duplicate-version,
wildcard, and source policies. `osv-scanner.toml` carries the matching documented
advisory filters. Four longstanding exceptions are limited to identified
transitive packages in PocketIC test tooling, Candid build/proc-macro support, or
the certified informational frontend. Their reasons are recorded beside each
advisory. They are not generic vulnerability classifications and must be
re-reviewed when dependency paths change.

The gate scans both locked Rust and npm dependency sets. It does not build or deploy canisters, mutate mainnet state, or replace the separate behavioural and reproducibility gates.

## Retired block-buffer applicability resolution

On 2026-10-30 GitHub merged
[advisory-database PR #10755](https://github.com/github/advisory-database/pull/10755),
and OSV propagated GHSA-qwgh-2vcv-g2f7 with the corrected affected interval
`>=0.11.0,<0.12.1`. The repository locks `block-buffer` 0.10.4, so the standard
OSV scan now classifies it as outside the affected range without a local filter.
The temporary GHSA entry and its exact-lockfile guard were therefore removed.

The historical decision remains documented in the
[retired exception record](../codex/block-buffer-not-affected-exception-proposal.md),
[source review](../codex/block-buffer-advisory-correction-request.md), and
[investigation](../codex/block-buffer-osv-investigation.md). Those records explain
the separate-cursor implementation, exact reviewed archive identity, production
dependency paths, and limits of the native caught-panic probe. They are evidence
for the earlier policy decision, not active scanner configuration.
