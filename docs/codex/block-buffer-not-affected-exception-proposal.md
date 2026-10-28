# Unapproved, narrowly scoped not-affected proposal

Status: **proposal only**. No security file or scanner result is changed. Explicit
operator approval is required before implementation. Correcting authoritative
metadata remains preferred; release security remains failed meanwhile.

Proposed applicability is the intersection of:

- Advisory GHSA-qwgh-2vcv-g2f7 only.
- Cargo package block-buffer exactly 0.10.4 in the root Cargo.lock.
- Registry archive checksum
  `3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71`.
- The reviewed separate-cursor implementation at RustCrypto/utils commit
  `6d35952d3d3b124bc1049ad6fb406b42b1ce4bfe`, path block-buffer.

Evidence and limitations: [independent source review and reproducible panic probe](block-buffer-advisory-correction-request.md).
The described caught-panic inline-cursor corruption was not found in that exact
release. This does not assert general memory safety or classify other releases.

A future approved implementation must fail closed when the package version,
source/checksum or lockfile path changes. An advisory-wide ignore alone does not
express this scope and is not this proposal. If the pinned scanner cannot express
these conditions, a separate reviewed applicability guard would be necessary;
none is implemented here. Preserve Cargo audit, Cargo deny, npm audit and online
OSV scanning, and expose every other finding. The four existing exceptions stay
unchanged. Remove the temporary resolution when authoritative metadata permits
the ordinary gate to pass. A proposal or approval alone is not a passing scan.
