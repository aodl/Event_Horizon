# Developer experience validation

## Revisions and scope

- Starting commit: `910f75744057253aa15b080e4d54f99b0a836a60`
- Validated tooling/documentation commit: `c870b4bb324b9cbc4641cce0d7a91ca096c0954c`
- Final evidence commit: the commit containing this report and the regenerated `MANIFEST.sha256`; its SHA is recorded in the final handoff because a Git commit cannot contain its own object ID.
- Branch: `codex/developer-experience`
- Protocol, production Candid, frontend behavior, permanent IDs, canister settings, surplus destination, and Dockerfile semantics were unchanged.

Changed files before evidence generation:

```text
README.md
docs/deployment.md
docs/reproducible-builds.md
docs/testing.md
tools/scripts/build-release
tools/scripts/docker-build
tools/scripts/icp-build-canister
tools/scripts/local-smoke (removed)
tools/scripts/security-scan
tools/scripts/verify-reproducible-artifacts
tools/static-check.py
tools/xtask/README.md (added)
tools/xtask/src/main.rs
```

This report and the root `MANIFEST.sha256` complete the evidence commit. Historical records under `docs/codex/` and `docs/provenance/checkpoint-03-MANIFEST.sha256` were otherwise left unchanged.

## Developer command surface

The final public `xtask` surface is:

```text
help
unit
check
pocketic
test-all
security
release
canonical
repro
validate
```

No-argument, `help`, `--help`, and `-h` forms all printed concise help and exited successfully. An unknown command printed an explicit error plus the same help and exited with status 2.

The redundant `local-smoke` script, dispatch branch, and current/live documentation references were removed. Historical records were preserved.

`test-all` runs `check` followed by the explicitly selected, serialized PocketIC suite. It does not repeat `unit` and performs no security, release, or Docker work.

`validate` runs `test-all`, `security`, `repro`, then `canonical`. The canonical build deliberately runs last so successful validation leaves exact deployable artifacts under `release-artifacts/`.

## Console and deployment visibility

- `xtask` prints phase headings while retaining underlying subprocess output.
- `security-scan` identifies all four scanner stages and prints `Security gate passed`.
- `build-release` identifies its output as host-toolchain artifacts, warns that it is not canonical evidence, and prints freshly calculated module hashes and the manifest path.
- `docker-build` verifies the manifest and prints source revision, working-tree state, both actual Wasm hashes, the full manifest path, and the independent verification command.
- `verify-reproducible-artifacts` prints the matching Wasm hashes from a compared temporary output set and explains that it does not populate `release-artifacts/`.
- In canonical-artifact mode, `icp-build-canister` verified the manifest and printed the exact artifact hash before copying it to `ICP_WASM_OUTPUT_PATH`. Both copied files independently matched the source artifact.

## Documentation

- Added `tools/xtask/README.md` with prerequisites, command matrix, underlying commands, and recommended workflows.
- Replaced the root README's flat validation list with a concise developer quick start.
- Rewrote `docs/testing.md` as a test/validation guide while retaining the PocketIC coverage inventory.
- Rewrote `docs/reproducible-builds.md` to distinguish local release, canonical production artifacts, same-environment determinism, and full validation.
- Expanded `docs/deployment.md` with the operator gate, canonical handoff, artifact hashes, and explicit install/verification lifecycle.
- Corrected the stale live empty-service example: `get_pricing` is the backend's sole production application query.
- Audited tracked live documentation for stale `local-smoke`, empty-service, no-application-method, injected-binding, `ic_env`, and `PUBLIC_CANISTER_ID:event_horizon` claims.

## Validation results

### Tests

- Backend Rust unit tests: 46 passed.
- Frontend JavaScript tests: 13 passed.
- PocketIC integration tests: 37 passed, serialized with no separately managed replica.
- Formatting, warnings-denied Clippy, ordinary workspace tests, static invariants, and frontend tests passed through `check`.
- `DFX_IDENTITY=codex_local icp build -e local` passed. The active DFX identity name was confirmed as `codex_local`; no identity material was inspected or exported.

Behavioral counts match the reviewed starting revision.

### Security

`cargo audit`, `cargo deny check advisories licenses bans sources`, `npm audit --omit=dev`, and OSV Scanner all passed under the existing policy. RustSec reported the four already allowed unmaintained-dependency warnings; OSV Scanner applied the existing documented filters and reported no issues. No filtering or policy was changed.

### Reproducibility and canonical artifacts

The first validation attempts exposed a full Docker build-cache filesystem, not a source/build defect. Only fully reclaimable Docker builder cache was pruned; no Dockerfile or build semantics changed. The final `validate` then passed end-to-end.

Two independent `docker build --no-cache` output sets matched for every emitted artifact. `repro` did not populate `release-artifacts/`. The final canonical build verified its manifest and left the deployable artifacts in place.

Canonical uncompressed Wasm SHA-256 values:

```text
d5a66897cb914488df53799842a89912f65a0688793b3eb198b064ed7d5c202b  event_horizon.wasm
3f19289a19a940ed023d05957a24c3e2d0b50402ddfffd978f08daa7aa01df63  event_horizon_frontend.wasm
```

Both values are unchanged from the reviewed mainnet-binding commit. Changes to build metadata reflect tooling inputs and do not change either production module.

### Source manifest and repository state

`tools/scripts/source-manifest generate` was run after source, documentation, and this evidence were complete. `tools/scripts/source-manifest verify` passed. `docs/provenance/checkpoint-03-MANIFEST.sha256` was not modified.

The branch was clean after the evidence commit. No deployment, top-up, mainnet setting, controller, surplus destination, or `X` publication action was performed.
