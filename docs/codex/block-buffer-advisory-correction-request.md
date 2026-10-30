# Submitted advisory range correction

Status: submitted as [advisory-database PR #10755](https://github.com/github/advisory-database/pull/10755), open at the 2026-10-29 resumed inspection. Authoritative metadata is not yet corrected. No scanner exception is approved or activated; do not submit a duplicate correction.

Target: GitHub Advisory Database
`advisories/github-reviewed/2026/08/GHSA-qwgh-2vcv-g2f7/GHSA-qwgh-2vcv-g2f7.json`.
The submitted request follows the [Suggest improvements process](https://docs.github.com/en/code-security/security-advisories/working-with-global-security-advisories-from-the-github-advisory-database/editing-security-advisories-in-the-github-advisory-database)
and proposes the following change for the Rust package `block_buffer`:

```diff
- "introduced": "0"
+ "introduced": "0.11.0"
```

Keep `fixed: 0.12.1`, severity, references and other advisory fields unchanged.
The [maintainer advisory](https://github.com/RustCrypto/utils/security/advisories/GHSA-qwgh-2vcv-g2f7)
explicitly lists `>=0.11.0,<0.12.1`. The reviewed database currently introduces at
zero; [OSV](https://api.osv.dev/v1/vulns/GHSA-qwgh-2vcv-g2f7) imports that file.
Retrieved metadata: GitHub updated 2026-08-19T19:15:35Z; OSV modified
2026-08-19T19:30:06.686866977Z. Neither source was corrected when checked.

The locked crate is **block-buffer 0.10.4**, archive SHA-256
`3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71`,
VCS commit `6d35952d3d3b124bc1049ad6fb406b42b1ce4bfe`, path `block-buffer`.
[Exact source](https://github.com/RustCrypto/utils/blob/6d35952d3d3b124bc1049ad6fb406b42b1ce4bfe/block-buffer/src/lib.rs).

The conclusion is based on all callback-bearing paths, not just struct shape:

- `digest_blocks` copies input into the byte array, then invokes `compress` with
  immutable blocks. Its separate cursor remains the previous valid value until
  the final bounded leftover length is assigned. A caught compression panic may
  change buffered bytes, but cannot overwrite that cursor with attacker bytes.
- The later `ReadBuffer` type and `write_block` API do not exist in 0.10.4.
  Its analogous eager `set_data` generator receives external blocks or a local
  temporary block. A generator panic occurs before any new cursor assignment.
- `digest_pad` callbacks receive immutable blocks. The cursor remains valid if
  either callback panics. `set` validates the position before mutation; constructors
  check the sealed Eager/Lazy invariant and block sizes are below 256. Reset and
  zero-padding write zero. Lazy digest callbacks likewise preserve the separate
  old cursor, including the valid full-block position.
- Unsafe slice conversions and the `get_pos` unreachable assertion were inspected;
  no alternate safe callback route that corrupts the position was identified.
  This is not a claim of arbitrary memory-safety or cryptographic recovery after
  panic, nor an audit of other vulnerabilities.

A native caught-panic probe exercised 191 cases over eager digest/padding/generation
and lazy digest, including 0xff overwrites and subsequent reads. Cursor invariants
held. This is supplementary evidence, **not Miri** or a universal proof.
Reproduce with `cargo run --locked --manifest-path docs/codex/release-closure-data/panic-probe/Cargo.toml`.
The archive checksum and fetched advisory JSON are retained in release-closure-data.

The graph is `block-buffer 0.10.4 → digest 0.10.7 → sha2 0.10.9`, consumed by backend
hashing, Candid/ic_principal, frontend certification and native PocketIC tooling.
No production dependency migration is proposed. Please have the maintainer confirm
that the database range should match the published affected interval. Do not mark
additional versions unaffected based solely on this 0.10.4 inspection; boundary
support for the proposed interval comes from the maintainer's explicit statement.

If an interim policy exception is needed, the separately reviewable proposal is
only GHSA-qwgh-2vcv-g2f7 for exactly locked block-buffer 0.10.4 with the checksum above,
subject to operator approval and removal after authoritative correction. This
proposal grants no approval and changes no policy file. The normal gate remains
failed until corrected data or an explicitly approved policy resolution passes it.
