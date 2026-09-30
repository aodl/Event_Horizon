# Event Horizon xtask

`xtask` is the preferred behavioural-test orchestrator. Release, security, reproducibility, formatting, and Clippy remain explicit scripts or tool commands.

## Public commands

| Command | Meaning |
| --- | --- |
| `frontend_setup` | Run locked `npm ci` only when frontend dependencies are absent or stale |
| `test_unit` | Static/source validation, source manifest, Rust workspace (including xtask), and frontend Node tests |
| `test_pocketic_integration` | Discover and run every ignored Event Horizon PocketIC scenario separately and serially |
| `test_all` | `test_unit` plus `test_pocketic_integration`, with one consolidated result |

```bash
cargo run -p xtask -- frontend_setup
cargo run -p xtask -- test_unit
cargo run -p xtask -- test_pocketic_integration
POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all
```

The managed PocketIC command sets `POCKET_IC_MUTE_SERVER=1` itself; setting it in the final example also documents the quiet full-acceptance convention used by Jupiter Faucet.

PocketIC execution discovers ignored library tests through libtest and then
runs each exact name in its own Cargo test process. Each scenario therefore has
a fresh process and PocketIC lifecycle. This provides precise per-test
reporting and avoids cumulative shared-server state across a long integration
run. Execution remains serial, zero-test discovery fails, and the managed
command also sets `RUST_TEST_THREADS=1`.

## Failure reporting

Each logical suite has captured combined output, live progress, duration, parsed test counts, status, and a rerun command. Every discovered PocketIC test is one independently recorded outcome, later tests continue after a failure, and the final summary includes the aggregate PocketIC pass/fail count. The final summary is always printed before a non-zero exit, and reproduces failing Rust test names, panic/error text, assertion `left:`/`right:` details, and precise isolated PocketIC reruns. A malformed summary or successful zero-test selection is a failure.

Each child command has a three-hour default ceiling covering execution and output collection. Set a positive integer `EVENT_HORIZON_TEST_COMMAND_TIMEOUT_SECS` to choose a different bound. Invalid values are rejected before the child starts. On Unix, stdout and stderr share one ordered capture stream; a timeout or a descendant retaining that stream after the direct child exits terminates the command's owned process group and performs only a bounded final drain.

When Rust's normal structured failure block is available, the summary extracts it. If an exact invocation instead emits an inline `--nocapture` panic or otherwise has no recognized block, the final failure detail contains the complete captured transcript rather than a tail. Timeout, spawn, capture, and parser diagnostics are retained alongside available output, and the known exact test name/rerun remain present.

## Direct commands

Source quality:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Release and security:

```bash
./tools/scripts/security-scan
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
```

The optional local-toolchain build remains `./tools/scripts/build-release`. Mainnet acceptance observation remains the read-only `./tools/scripts/mainnet-observe` command.

See [testing](../../docs/development/testing.md) and [reproducible builds](../../docs/operations/reproducible-builds.md).
