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

`deny.toml` enforces the repository's advisory, license, duplicate-version, wildcard, and source policies. `osv-scanner.toml` carries the matching documented advisory filters. The current exceptions are limited to identified transitive packages in PocketIC test tooling, Candid build/proc-macro support, or the certified informational frontend; their reasons are recorded beside each advisory in those files. They are not generic vulnerability classifications and must be re-reviewed when dependency paths change.

The gate scans both locked Rust and npm dependency sets. It does not build or deploy canisters, mutate mainnet state, or replace the separate behavioural and reproducibility gates.
