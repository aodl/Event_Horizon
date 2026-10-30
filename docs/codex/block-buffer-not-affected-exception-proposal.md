# Narrowly scoped not-affected implementation for operator review

Status: **implemented locally for operator review, not approved for merge**.
Authoritative metadata remained erroneous on 2026-10-30 and correction
[PR #10755](https://github.com/github/advisory-database/pull/10755) remained open.
The operator must explicitly approve this security-policy change before merging
Event Horizon PR #12.

Implemented applicability is the intersection of:

- Advisory GHSA-qwgh-2vcv-g2f7 only.
- Cargo package block-buffer exactly 0.10.4 in the root Cargo.lock.
- Source exactly
  `registry+https://github.com/rust-lang/crates.io-index`.
- Registry archive checksum
  `3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71`.
- Exactly one hyphen/underscore-normalised matching locked package.
- The reviewed separate-cursor implementation at RustCrypto/utils commit
  `6d35952d3d3b124bc1049ad6fb406b42b1ce4bfe`, path block-buffer.

Evidence and limitations: [independent source review and reproducible panic probe](block-buffer-advisory-correction-request.md).
The described caught-panic inline-cursor corruption was not found in that exact
release. This does not assert general memory safety or classify other releases.

OSV's ignore is advisory-wide, so `tools/scripts/block-buffer-osv-guard` runs its
regression suite and then parses the actual root `Cargo.lock` before OSV starts.
It uses the standard-library TOML parser and accepts no lockfile override. It
fails on malformed input, absence, duplicates, alternate spelling or any field
change. Synthetic tests reject 0.11.0, 0.12.0 and even fixed 0.12.1 until the
exception is removed; this makes dependency movement an explicit policy event.
Integration tests show a guard failure prevents a filtered OSV success and an
unrelated OSV failure still fails the canonical command.

Cargo audit, Cargo deny, npm audit and the online OSV scan remain unchanged and
active. The four previous OSV entries are unchanged. No Cargo dependency,
lockfile, severity threshold or production code is changed. The dependency is
not dev-only: digest/SHA-2 reaches backend hashing, Candid principals and frontend
certification. A RustCrypto-family migration would change that production graph
solely to suppress the imported range discrepancy and was not selected.

This implementation is a temporary local policy resolution. A green result is
green under the exact guarded exception, not an upstream-corrected or exception-
free result. After GitHub and OSV correct the range, remove the GHSA entry and
this otherwise-unused guard/tests, run unfiltered OSV, and require successful CI
without the exception. The repository owner is responsible for that removal.
