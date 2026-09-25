# Reproducible builds

The backend is intended to become controllerless, so source-to-Wasm reproducibility is part of Event Horizon's trust model.

| Goal | Command |
| --- | --- |
| Fast logic tests | `cargo run -p xtask -- unit` |
| Normal code check | `cargo run -p xtask -- check` |
| Integration suite | `cargo run -p xtask -- pocketic` |
| All behavior/source tests | `cargo run -p xtask -- test-all` |
| Security dependencies | `cargo run -p xtask -- security` |
| Local release Wasms | `cargo run -p xtask -- release` |
| Canonical deployable Wasms | `cargo run -p xtask -- canonical` |
| Deterministic rebuild proof | `cargo run -p xtask -- repro` |
| Everything needed before deployment | `cargo run -p xtask -- validate` |

## Local release build

```bash
cargo run -p xtask -- release
```

This invokes `tools/scripts/build-release` with the host Rust, npm, and `ic-wasm` environment. It is useful for development and quick release inspection, but is not canonical source-to-Wasm verification.

## Canonical Docker build

```bash
cargo run -p xtask -- canonical
```

This is equivalent to `./tools/scripts/docker-build`. `Dockerfile.repro` pins the Debian base-image digest and snapshot, Rust toolchain, rustup installer digest, `ic-wasm`, `Cargo.lock`, and `package-lock.json`. Source paths are remapped and both production modules pass through the pinned `ic-wasm shrink` step.

The command verifies the artifact manifest, prints the source revision, working-tree state, and exact Wasm hashes, and produces:

```text
release-artifacts/event_horizon.wasm
release-artifacts/event_horizon_frontend.wasm
release-artifacts/release-artifacts.sha256
release-artifacts/build-info.json
```

Event Horizon releases uncompressed `.wasm` files. Therefore `SHA-256(release-artifacts/event_horizon.wasm)` and `SHA-256(release-artifacts/event_horizon_frontend.wasm)` are intended for direct comparison with their installed mainnet module hashes; no `.wasm.gz` representation is involved.

For independent inspection after the build:

```bash
(
  cd release-artifacts
  sha256sum -c release-artifacts.sha256
)
```

The canonical build already performs this verification and fails on any mismatch.

## Reproducibility proof

```bash
cargo run -p xtask -- repro
```

This performs two independent `docker build --no-cache` artifact builds and compares every emitted file hash.

This proves same-environment determinism. It does not by itself populate `release-artifacts/` and does not compare against mainnet. Run `canonical` when deployable artifacts are required.

## Full pre-deployment validation

```bash
cargo run -p xtask -- validate
```

This intentionally expensive gate runs all source/behavior tests, dependency-security policy, the two-build reproducibility proof, and finally the canonical build. It ends with exact deployable artifacts under `release-artifacts/`.

## Production-surface audit

`tools/audit-wasm.py` parses the Wasm export section directly. The release build fails unless the backend's sole production application query is `get_pricing`; it also fails if debug markers occur in the production module. The frontend must expose exactly `canister_query http_request` as its application method and must not expose `http_request_update`, pricing proxies, or debug methods. The canonical build runs the pinned JavaScript bundle step before compiling the embedded frontend Wasm.

This supplements, rather than replaces, the checked-in production DID, whose sole application method is:

```candid
service : {
  get_pricing : () -> (Pricing) query;
}
```

## Mainnet verification

After deployment, compare each public module hash reported by ICP with the SHA-256 printed by `canonical`. Retain the exact source revision, `build-info.json`, and artifact manifest used for installation. Repeat the backend comparison immediately before controller removal.

Current primary guidance: [ICP reproducible builds](https://docs.internetcomputer.org/guides/canister-management/reproducible-builds/).
