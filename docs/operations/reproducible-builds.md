# Reproducible builds

`Dockerfile.repro` defines the canonical production build environment. Event Horizon deliberately releases uncompressed `event_horizon.wasm` and `event_horizon_frontend.wasm`; it does not use Jupiter Faucet's `.wasm.gz` convention.

## Canonical gates

```bash
./tools/scripts/security-scan
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
```

Both canonical wrappers resolve `HEAD` once, reject tracked modifications and non-ignored untracked source, verify `MANIFEST.sha256`, export that exact revision with `git archive`, and verify the manifest again inside a temporary context outside the working tree. Ignored files, caches, credentials, `node_modules`, `target`, local ICP state, observations, and existing artifacts therefore cannot enter the build context. The resolved commit is printed. `verify:reproducible-artifacts` prepares one such frozen context and uses it for both independent `docker build --no-cache` builds, hashes every emitted file, and requires byte-identical outputs. It uses temporary outputs and does not populate `release-artifacts/`.

`docker-build` builds once in the same pinned environment, replaces `release-artifacts/` with the deployable files, verifies `release-artifacts.sha256`, and prints the two module hashes. Verify the resulting manifest independently with:

```bash
(
  cd release-artifacts
  sha256sum -c release-artifacts.sha256
)
```

The canonical build produces all four release outputs:

```text
release-artifacts/event_horizon.wasm
release-artifacts/event_horizon_frontend.wasm
release-artifacts/release-artifacts.sha256
release-artifacts/build-info.json
```

`Dockerfile.repro` pins the Debian base-image digest and snapshot, Rust toolchain, rustup installer digest, `ic-wasm`, `Cargo.lock`, and `package-lock.json`. It removes inherited APT source files, points APT's source-parts directory at an explicit empty directory, and uses only the signed Debian and Debian Security snapshot entries in `/etc/apt/sources.list`; package-signature verification remains enabled. Source paths are remapped and both modules pass through the pinned `ic-wasm shrink` step. Both build scripts stage output separately and replace an existing `release-artifacts/` only after a successful build and manifest verification. A failed Docker build or artifact-manifest check leaves a prior valid directory intact.

Event Horizon releases uncompressed `.wasm` files. Therefore the SHA-256 values of `event_horizon.wasm` and `event_horizon_frontend.wasm` are compared directly with installed ICP module hashes; no `.wasm.gz` representation is involved.

The optional host-toolchain build is:

```bash
./tools/scripts/build-release
```

It is useful for local inspection but is not canonical reproducibility evidence. The split is intentional: xtask orchestrates behavioural testing, while scripts own security, reproducibility, and artifact production.

## Production-surface audit

`tools/audit-wasm.py` parses the Wasm export section directly. The build fails unless the backend application surface consists of exactly these two queries:

```candid
service : {
  get_instance : () -> (InstanceInfo) query;
  get_pricing : () -> (Pricing) query;
}
```

The audit rejects backend update methods and debug markers. The frontend must expose exactly `canister_query http_request`, with no HTTP update, pricing proxy, or administrative application method. These binary audits supplement rather than replace the checked-in Candid interfaces.

The backend build has no surplus-recipient environment variable, build argument,
feature, source substitution, or recipient-specific Docker path. One backend
module hash covers every valid install-time `surplus_canister`. The frontend is
expected to change when its reviewed static registry pins a different backend
hash or instance recipient; that certified registry change does not create a
recipient-specific backend Wasm.

## Release and mainnet evidence

Retain together:

- the exact clean source revision used by the canonical build;
- `build-info.json`, which records pinned build inputs and tool versions;
- `release-artifacts.sha256` and all files it authenticates;
- the two-build reproducibility result;
- the backend and frontend module hashes observed on ICP;
- the source manifest and validation report for that release.

After deployment, compare each public module hash with the SHA-256 printed by `./tools/scripts/docker-build`. Before controller removal, rerun the complete source, security, reproducibility, export-audit, and manifest checks from the retained source revision, then repeat the live backend module-hash comparison. Controller removal must never rely on an earlier build from a dirty or different tree.

Canonical deployment requires a clean reviewed source revision, matching two-build evidence, audited Wasm exports/imports, a verified artifact manifest, and operator comparison of uncompressed module hashes. See [deployment](deployment.md).

Current primary guidance: [ICP reproducible builds](https://docs.internetcomputer.org/guides/canister-management/reproducible-builds/).
