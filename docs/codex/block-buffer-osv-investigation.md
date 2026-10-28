# block-buffer OSV metadata discrepancy

The fresh cycle-optimisation scan failed OSV on `GHSA-qwgh-2vcv-g2f7` for locked
`block-buffer 0.10.4`. Cargo audit, deny and npm audit passed their existing policies;
Tokio remains fixed at 1.51.6. The overall security gate is **failed**, not waived.

The [maintainer advisory](https://github.com/RustCrypto/utils/security/advisories/GHSA-qwgh-2vcv-g2f7)
lists affected versions `>=0.11.0,<0.12.1`. The [OSV imported record](https://osv.dev/vulnerability/GHSA-qwgh-2vcv-g2f7)
instead has an introduced event of `0`, fixed `0.12.1`, for `block_buffer`.
Retrieved OSV JSON SHA-256:
`a35c62683978eb74c07ee32a1a78c303ca81869fbd6856509e3f6dce68d11105`.

`cargo tree --locked -i block-buffer` identifies digest 0.10.7 → sha2 0.10.9,
including backend hashing, Candid principals, frontend certification and native
PocketIC tooling. The locked 0.10.4 source has `BlockBuffer { buffer, pos: u8, ... }`,
with the cursor separate from the data. The advisory's corrupted inline cursor is
part of the later EagerBuffer/ReadBuffer design. The maintainer's version boundary
and the actual locked implementation support classifying this as a downstream
metadata mismatch, rather than upgrading the entire IC/Candid dependency graph.

No advisory exclusion, scanner override, source patch or dependency change was
made for this finding. No external issue/PR was published. Security release
acceptance remains blocked until authoritative scanner data is corrected or an
explicitly reviewed resolution is available. A fresh scan at validated source `7cfa8b3`, using workflow-pinned cargo-audit
0.22.1 and OSV 2.3.8, reproduces this sole unfiltered finding; audit/deny/npm pass.
The branch is now published at d13d113. Remote CI remains unverified because
PR creation returned403 and no relevant PR run was found; see the
[release-closure follow-up](cycle-optimisation-release-closure.md). Raw scan and JSON are in the normal observations
area; reproduce with `./tools/scripts/security-scan` and the linked public data.

Independent closure review,191native caught-panic cases, fresh machine metadata
and a precise ready correction request are preserved in the follow-up. No
authoritative correction or activated exception is claimed.
