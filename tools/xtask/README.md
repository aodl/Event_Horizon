# Event Horizon developer commands

## Purpose

`xtask` is the preferred entry point for testing and release validation. It knows which tests are intentionally ignored, which integration package to invoke, which repository-specific static checks apply, and which security and release scripts define the acceptance gates.

Run `cargo run -p xtask -- help` for concise command-line help.

Mainnet acceptance snapshots are deliberately outside deterministic validation:

```sh
./tools/scripts/mainnet-observe [backend-id] [frontend-id]
```

This command is read-only and may use controller credentials solely to read cumulative `canister_metrics`. It performs no deployment or settings mutation.

## Prerequisites

### Normal development

- Rust and Cargo, including the `wasm32-unknown-unknown` target
- Node.js and npm
- Python 3

Install locked frontend dependencies once with `npm ci`.

### Integration

PocketIC is supplied through the Rust dependency and test harness. It does not require a separately managed local IC replica.

### Security

- `cargo-audit`
- `cargo-deny`
- `osv-scanner`
- npm

### Release and reproducibility

- Docker for `canonical`, `repro`, and `validate`
- `ic-wasm` for the host-toolchain `release` command

The canonical Docker build uses its pinned Rust, Node/npm, and `ic-wasm` environment for the Wasms; it does not rely on those host versions.

### Deployment tooling

Install the `icp` CLI for `icp build` and operator-controlled deployments.

## Command matrix

| Command | Meaning | Typical use |
| --- | --- | --- |
| `unit` | Backend/frontend unit tests | Fast iteration |
| `check` | Source/quality gate | Normal pre-commit |
| `pocketic` | Full integration suite | Behavior/integration |
| `test-all` | `check` + PocketIC | Strong local confidence |
| `security` | Dependency security | Release/security |
| `release` | Local-toolchain Wasms | Quick local release inspection |
| `canonical` | Canonical Docker Wasms | Deployable artifacts |
| `repro` | Two clean builds | Deterministic rebuild proof |
| `validate` | All pre-deploy gates | Before deployment |

## What each command actually executes

### `unit`

Fast behavioral tests for backend domain logic and frontend JavaScript:

```bash
cargo test --locked -p event-horizon --lib
npm test
```

It deliberately omits formatting, Clippy, PocketIC, security checks, and release builds.

### `check`

The normal pre-commit/source-quality gate, in this order:

```bash
python3 tools/static-check.py
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
npm test
```

This verifies static protocol/source invariants, formatting, warnings-denied Clippy, ordinary workspace Rust tests, and frontend Node tests. Ordinary Cargo testing skips the intentionally ignored PocketIC scenarios.

### `pocketic`

The full deterministic integration suite:

```bash
cargo test --locked -p event-horizon-pocketic -- --ignored --nocapture --test-threads=1
```

The scenarios are `#[ignore]` under ordinary Cargo tests and are invoked explicitly here. They run serially for deterministic canister and failure-state testing. No separately managed replica is required. The reviewed starting revision has 37 scenarios; the count may increase as coverage grows.

### `test-all`

Runs `check`, then `pocketic`. It does not repeat `unit`, because `check` already runs the ordinary backend and frontend tests. Use it for all behavioral/source tests without release, security, or Docker work.

### `security`

Runs `tools/scripts/security-scan`, which executes:

```bash
cargo audit
cargo deny check advisories licenses bans sources
npm audit --omit=dev
osv-scanner scan -L Cargo.lock -L package-lock.json
```

### `release`

Runs `tools/scripts/build-release`. This builds and audits both production Wasms with the host Rust/npm/`ic-wasm` toolchain and writes `release-artifacts/`. It is useful for local inspection, but is not canonical reproducibility evidence.

### `canonical`

Runs `./tools/scripts/docker-build`. It builds in the pinned Docker environment, verifies the artifact manifest, leaves the exact deployable Wasms under `release-artifacts/`, and prints their module hashes.

### `repro`

Runs `tools/scripts/verify-reproducible-artifacts`. It performs two independent `docker build --no-cache` builds and compares every emitted artifact. This proves same-environment determinism, uses temporary outputs, and does not populate `release-artifacts/`.

### `validate`

The intentionally expensive pre-deployment gate runs, in order:

1. `test-all` (`check`, then PocketIC)
2. `security`
3. `repro`
4. `canonical`

The canonical build runs last so a successful validation leaves the deployable artifacts in `release-artifacts/`. It does not run the redundant host-toolchain `release` build.

## Recommended workflows

### Fast edit loop

```bash
cargo run -p xtask -- unit
```

### Before committing

```bash
cargo run -p xtask -- check
```

### Strong behavioral confidence

```bash
cargo run -p xtask -- test-all
```

### Before production deployment

```bash
cargo run -p xtask -- validate
```

### Rebuild exact production Wasms only

```bash
cargo run -p xtask -- canonical
```
