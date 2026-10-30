# Retired narrowly scoped not-affected implementation

Status: **historical; the temporary exception and guard are removed**.

Event Horizon PR #12 temporarily carried this exact-version policy while the
imported advisory range was incorrect. On 2026-10-30 GitHub merged correction
[PR #10755](https://github.com/github/advisory-database/pull/10755), and OSV then
published GHSA-qwgh-2vcv-g2f7 with the corrected affected interval
`>=0.11.0,<0.12.1`. Because locked `block-buffer` 0.10.4 is outside that range,
the repository returned to the standard OSV scan without this local exception.

The following records the retired implementation and its review boundary; it is
not current scanner policy.

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

OSV's ignore was advisory-wide, so the removed `block-buffer-osv-guard` ran its
regression suite and then parsed the actual root `Cargo.lock` before OSV started.
It used the standard-library TOML parser and accepted no lockfile override. It
failed on malformed input, absence, duplicates, alternate spelling or any field
change. Synthetic tests rejected 0.11.0, 0.12.0 and even fixed 0.12.1 until the
exception was removed, making dependency movement an explicit policy event.
Integration tests showed a guard failure prevented a filtered OSV success and an
unrelated OSV failure still failed the canonical command.

Cargo audit, Cargo deny, npm audit and the online OSV scan remained active. The
four other OSV entries were unchanged. No Cargo dependency, lockfile, severity
threshold or production code changed. The dependency was not dev-only:
digest/SHA-2 reaches backend hashing, Candid principals and frontend
certification. A RustCrypto-family migration would have changed that production
graph solely to suppress the imported range discrepancy and was not selected.

While active, this was a temporary local policy resolution: a green result was
green under the exact guarded exception, not an upstream-corrected or
exception-free result. The correction and cleanup now make that distinction
historical; the current gate no longer contains the GHSA entry or guard.
