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
./tools/scripts/block-buffer-osv-guard --test
./tools/scripts/block-buffer-osv-guard
osv-scanner scan -L Cargo.lock -L package-lock.json
```

`deny.toml` enforces the repository's advisory, license, duplicate-version,
wildcard, and source policies. `osv-scanner.toml` carries the matching documented
advisory filters. Four longstanding exceptions are limited to identified
transitive packages in PocketIC test tooling, Candid build/proc-macro support, or
the certified informational frontend. The fifth temporary entry is constrained
by the exact lockfile guard documented below. Their reasons are recorded beside
each advisory. They are not generic vulnerability classifications and must be
re-reviewed when dependency paths change.

The gate scans both locked Rust and npm dependency sets. It does not build or deploy canisters, mutate mainnet state, or replace the separate behavioural and reproducibility gates.

## Temporary block-buffer applicability resolution

GitHub and OSV currently import GHSA-qwgh-2vcv-g2f7 as affecting every
`block_buffer` release before 0.12.1. The
[RustCrypto maintainer advisory](https://github.com/RustCrypto/utils/security/advisories/GHSA-qwgh-2vcv-g2f7)
instead identifies `>=0.11.0,<0.12.1`. The upstream range correction is already
submitted as [advisory-database PR #10755](https://github.com/github/advisory-database/pull/10755).
The local ignored-vulnerability entry is a temporary policy resolution, not an
upstream correction and not a general assertion that `block-buffer` is safe.

The exception is applicable only to exactly one root `Cargo.lock` package with
all of these fields:

| Field | Required value |
| --- | --- |
| name | `block-buffer` |
| version | `0.10.4` |
| source | `registry+https://github.com/rust-lang/crates.io-index` |
| checksum | `3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71` |

Before OSV applies its advisory-wide filter, `block-buffer-osv-guard` parses the
actual repository root lockfile with Python's standard-library TOML parser. It
normalises hyphen/underscore spelling for discovery, requires exactly one match,
and then requires the exact reviewed spelling and identity above. A missing,
additional, moved, malformed or changed package fails the complete gate. Its
synthetic tests cover the exact pass case, versions 0.11.0/0.12.0/0.12.1, source
and checksum changes, removal, duplication (including underscore spelling) and
malformed TOML. Shell-integration tests prove a guard failure prevents OSV from
running and a later OSV finding remains fatal.

The reviewed 0.10.4 implementation at RustCrypto/utils commit
`6d35952d3d3b124bc1049ad6fb406b42b1ce4bfe` keeps its cursor separate from the
buffer. A callback panic can alter buffered bytes but cannot overwrite the cursor
with attacker-controlled inline data. The later `ReadBuffer` and `write_block`
surface named by the advisory is absent. The retained native probe exercised 191
caught-panic cases, but is neither Miri nor proof of arbitrary memory safety or
cryptographic recovery. See the [source review](../codex/block-buffer-advisory-correction-request.md)
and [investigation](../codex/block-buffer-osv-investigation.md).

This crate remains on production paths through digest/SHA-2, backend hashing,
Candid principals and frontend certification; it is not classified as dev-only.
Upgrading the RustCrypto family would perturb the wider IC/Candid production
dependency graph solely to accommodate erroneous imported metadata, so it was
not selected for this release.

A green gate with this entry is **green under the exact reviewed local
exception**, not green without exceptions. When GitHub and OSV correct the range,
the owner must remove the GHSA-specific `IgnoredVulns` entry and the now-unused
guard/tests, run the ordinary unfiltered OSV command, and confirm CI passes with
no local exception. Do not retain the entry merely because it has become inert.
