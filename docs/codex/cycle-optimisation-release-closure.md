# Cycle optimisation release closure

Local engineering validation is complete. **Release clearance remains blocked**
by the unsuppressed advisory metadata finding and unavailable PR creation/CI
permissions. No deployment, real transfer, controller action or merge occurred.

## Sources and scope

Starting candidate: `9528da787f9e8a4d5df2d297401564d04adb41b6`.
Validated build recipe: `d13d1138e2fde4af18bd0d3f51f777470037f93f`.
The subsequent evidence commit changes docs/data/manifest only; its exact SHA is
Git HEAD and the final response. This avoids embedding a report's own hash.

The published branch `codex/cycle-optimisation-81c5a0d` was verified at d13d113;
main remains `81c5a0d72ff188f5ee9f412deb20566a12307642`. Earlier failures to publish
are historical observations. Publication does not establish CI or deployment approval.

The sole executable build change is `tools/scripts/build-release`. Production
Rust, Candid, dependencies/locks, assets, backend pin, stable IDs/encodings,
admission/notification logic, cadence and financial safeguards are unchanged.
The original [optimisation evidence](cycle-optimisation-validation.md), including
72 canonical comparison pairs and stable/upgrade/interleaving regressions, remains
valid because the final backend is **byte-identical**. No performance campaign
was rerun unnecessarily and no unconditional daily-burn saving is claimed.

## Compiler selection

Select backend `2` / frontend full-workspace `z`. The same two-package set is
built twice, preserving Cargo feature unification. The first invocation retains
the backend; the second uses isolated `target/frontend-size` output and retains
only the frontend. It intentionally compiles an unused backend-z intermediate.
This small build-time cost keeps the recipe explicit; no custom framework is added.
All target runtime dependencies receive the corresponding invocation's level;
Cargo's host/build-dependency defaults remain unchanged. Package-only z instead
optimises just the frontend package, leaving shared target dependencies at 2.

| Frontend option | Final bytes | Install/certification cycles | Reported idle/day | Allocated heap |
|---|---:|---:|---:|---:|
| Workspace 2 | 4,009,935 | 13,914,479,233 | 1,176,063,315 | 3,997,696 |
| Package-only z | 3,975,247 | 13,687,637,098 | 1,175,177,104 | 3,997,696 |
| Standalone z control | 3,757,926 | 12,460,781,460 | 1,169,624,979 | 3,997,696 |
| Selected workspace z | 3,757,901 | 12,460,593,460 | 1,169,624,340 | 3,997,696 |

All four options use identical current source/assets/pin and pass twenty certified
response checks each. Package-only z is rejected for its much smaller benefit.
Standalone frontend-only z changes dependency feature unification: its differing
hash/25 extra bytes demonstrate why neither control substitutes for full-workspace
z. Selected z reproduces the earlier full-z size and cost figures; its hash differs
from the earlier pre-pin frontend, whose assets legitimately differed.

Selected frontend saves 252,034 bytes (6.29%) and about 1.454B installation cycles
(10.45%) against workspace2. Installation is a one-time cost. Reported idle billing
falls about6.439M/day (0.55%); this is separate from recurring replicated work and
ordinary queries. Reported query statistics cover13 of 20 known calls and do **not**
support exact query-performance claims. Heap is unchanged. No `s` experiment was
needed after the useful z result and earlier z/s/2/3 screening.

A supplementary installed-canonical-module audit checks **all 29 public assets**
(including HTML, complete JS bundle, CSS, SVG, icons, images and well-known data)
on both workspace2 and hybrid frontends. All2,695,829body bytes match source;
certificate headers are present, queries succeed, stable memory is0 and allocated
heap is3,997,696 in each. This is separate from the twenty checks per profile;
encoded response lengths can differ because certificate proofs differ. See
[raw data and reproduction](release-closure-data/README.md).

## Security investigation

The maintainer's current range is `>=0.11.0,<0.12.1`. Current GitHub reviewed data
still says `<0.12.1`; OSV imports `introduced:0,fixed:0.12.1` from that database.
Fresh JSON retrieval receipts preserve their timestamps and digests. GitHub last
updated 2026-08-19T19:15:35Z; OSV modified 2026-08-19T19:30:06.686866977Z. Neither
correction nor upstream acceptance has been verified.

Independent review of exact locked 0.10.4 covered digest, generation, padding,
constructor/setter and lazy-buffer panic paths and unsafe cursor assumptions.
The separate cursor remains valid at callback panic points; the later ReadBuffer
API is absent.191 native caught-panic cases preserve cursor invariants. This
supports classifying the described issue as a range discrepancy for this exact
crate, not a universal memory-safety or post-panic hash-correctness proof. See
[the source-level correction request](block-buffer-advisory-correction-request.md)
for the checksum, VCS identity, precise database file and proposed0→0.11.0 change.
No unrelated version is classified solely from this 0.10.4 inspection.

The request is **ready to submit, not submitted**: available tooling has not
provided an authenticated advisory-improvement/fork submission path. No external
issue, PR or correction is invented. The documented GitHub Suggest improvements
form is the intended handoff. A [separate exact-version exception proposal](block-buffer-not-affected-exception-proposal.md)
is unapproved and unimplemented; it is not an advisory-wide ignore. No security
configuration, dependency version, severity or scan result was changed.

Fresh gate at d13d113: Cargo audit 0.22.1, Cargo deny 0.19.8 and npm audit 11.12.1
(Node 24.15.0) pass existing policy; OSV 2.3.8 **fails** solely on
GHSA-qwgh-2vcv-g2f7/block-buffer 0.10.4. All four existing filters remain unchanged.
RustSec database commit 40f5be6fb12ed93fc591bf4ab23b7d793b3f2239 is timestamped
2026-10-28T09:50:03+01:00. Advisory JSON was fetched online again during closure.
Tokio remains 1.51.6. The locked path is block-buffer 0.10.4 → digest 0.10.7 → sha2
0.10.9, used by backend hashing, Candid/principals, frontend certification and
native PocketIC. The scanner's failure is not waived by this technical assessment.

## Gates and canonical artifacts

At exact committed recipe d13d113:

- fmt and strict workspace/all-target/all-feature Clippy pass.
- test_unit passes146 Rust + 138 frontend; test_all passes 128 PocketIC: **412 total**.
- Randomized matching, logical stable regions 0–11, alias/price interleavings,
  failed-scan durability, baseline/historical upgrades, pending financial recovery,
  maximum subject-aware payloads and zero-argument Index compatibility remain green.
- Source manifest, static/canonical-context, production export and Candid audits pass.
- Supported `npm run verify:reproducible-artifacts` passes **two independent
  uncached Docker builds**, comparing every generated file. Population with the
  supported `./tools/scripts/docker-build` produces the matching Wasm identities;
  its three-entry artifact manifest verifies, including build-info.json.

| Canonical artifact | SHA-256 | Bytes |
|---|---|---:|
| Backend | `f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970` | 1,847,228 |
| Frontend | `210d4de6587a19228801f1d97264cb87d0c8d1117632ecb6f7051829b3b441c8` | 3,757,901 |

The registry, locked bundle and static check retain backend pin
`f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`.
Rust 1.94.1, Docker base digest/snapshot, dependency graph, LTO, codegen units,
panic/overflow/target policy, SOURCE_DATE_EPOCH and ic-wasm 0.9.7 shrink are preserved.
Docker metadata records Node 18.20.4/npm 9.2.0 separately from host security tools.
Command statuses, raw-log digests, source SHA, compiler rows and artifact metadata
are tracked in [closure receipts](release-closure-data/closure-receipt.json) and
[the full command ledger](release-closure-data/status.json). Large logs remain in
observations/release-closure/. Final evidence-only population is compared against
all four retained d13 outputs; its local receipt is evidence-final-status.json.

## GitHub and remaining actions

No associated PR was found. Creation targeting main was attempted again against
the now-published d13d113 branch and returned **403 Resource not accessible by
integration**. The GitHub CLI is unavailable; Git push lacked HTTPS credentials
in the preceding closure attempt. No PR number/URL or security run exists to report;
PR-workflow lookup for d13d113 returned no runs. The security workflow triggers
PRs or main/master pushes, not ordinary pushes to this branch.

An operator must publish the final evidence commit if necessary and use the
[ready PR title/body](cycle-optimisation-pr-handoff.md) for branch
`codex/cycle-optimisation-81c5a0d`→main. Then verify [Dependency Security](https://github.com/aodl/Event_Horizon/actions/workflows/dependency-security.yml)
for that exact head SHA, recording its actual result. No future/pending success is
claimed. Required clearance remains: corrected authoritative advisory data (or an
explicitly approved scoped policy resolution) yielding a passing normal gate,
and verified PR/CI for the final source. No merge or deployment is authorised.
