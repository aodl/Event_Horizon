# Reproducible builds

The backend is intended to become controllerless, so source-to-Wasm reproducibility is part of Event Horizon's trust model.

## Canonical build

```bash
./tools/scripts/docker-build
```

This uses `Dockerfile.repro`, which pins:

- Debian base-image digest;
- Debian snapshot date;
- Rust toolchain;
- rustup installer digest;
- `ic-wasm` version;
- `Cargo.lock` and `package-lock.json`.

Source paths are remapped before compilation and release artifacts are passed through a pinned `ic-wasm shrink` step.
The output is:

```text
release-artifacts/event_horizon.wasm
release-artifacts/event_horizon_frontend.wasm
release-artifacts/release-artifacts.sha256
release-artifacts/build-info.json
```

The Wasm files are deliberately left uncompressed so the published SHA-256 can be compared directly with the installed
module hash without gzip representation ambiguity.

## Same-environment double build

```bash
./tools/scripts/verify-reproducible-artifacts
```

This performs two clean Docker builds and compares every produced artifact byte-for-byte.

## Production-surface audit

`tools/audit-wasm.py` parses the Wasm export section directly. The release build fails if the backend exports any
`canister_update`, `canister_query`, or `canister_composite_query` application method, or if debug markers occur in the
production module. The frontend build must expose `canister_query http_request` and must not expose debug methods.

This supplements, rather than replaces, the checked-in production DID:

```candid
service : () -> {}
```

## Mainnet verification

After deployment, compare the public module hash reported by ICP against the SHA-256 of
`release-artifacts/event_horizon.wasm`. Retain the source revision, `build-info.json`, and artifact manifest used for the
install. Repeat the comparison immediately before controller removal.

Current ICP guidance: https://docs.internetcomputer.org/guides/canister-management/reproducible-builds/
