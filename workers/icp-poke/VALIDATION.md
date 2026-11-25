# Continuous worker validation

## Local checks, 25 November 2026

The artifact was rebuilt from Git base
`67096371f3d72c7a8bf473c00094d837d2f20a00` with local Nibrun checkpoint path
validation changes. `dist/BUILD.txt` records the exact build and installer
input digests. The compiler and embedded runtime are Bun
`1.4.5+a7733435c`, targeting `bun-linux-x64`. The build and source tests used
Node.js 24.15.0 and npm 11.12.1. The unchanged npm lockfile pins
`@icp-sdk/core` 6.1.0 and ten transitive packages. ELF inspection found the
x86-64 glibc loader and required symbol versions through GLIBC_2.17.
The build used the stable compile path with an isolated input tree. Artifact
identity is established by the hashes below and by comparing the executable
in the archive byte for byte. Build reproducibility was not evaluated.

| Output under `workers/icp-poke/dist/` | Bytes | SHA-256 |
| --- | ---: | --- |
| `eh-poke-linux-x86_64` | 109,619,376 | `e314094bee2b4556c3c4cfbbbd46f13a2ae197f441f96094bc5bdb619937643b` |
| `eh-poke-vps-linux-x86_64.tar.gz` | 51,283,558 | `84e524ec3a7f9be5188b82d04f1e0a1b298348de656f21840b4beb967d816e69` |
| `BUILD.txt` | 1,689 | `626788bf8b09cac9d2cf519edaca33dca68587269911b0b95934fb5b0908e4c2` |

This rebuild passed 74 Node source tests, 62 Bun source tests and 10 artifact
tests. The integrated VPS baseline had 71 source tests. The executable work
moved four installer/package tests to the artifact suite and added three
source cases, bringing that count to 70. This update adds four source cases
for Nibrun checkpoint containment.
[Build and test commands](README.md#build-and-test)

The Bun run repeats seven shared-module suites from the Node suite. They cover
accounts, config, matching, file storage, ICP SDK interaction, Candid and
poll timing. Node still runs the source CLI and launcher tests. The artifact
suite separately exercises the actual release executable and the compiled
harness. Release checks include offline validation, Nibrun checkpoint
containment, a bind conflict, startup and retry status, and graceful HTTP
shutdown in an isolated environment without an external runtime.
The harness verifies a signed zero-argument SDK request, rejects an HTTP 202
without certified acknowledgement, cancels real fixture transport, and
checks the durable file store. Signature settings and mocked SDK configuration
checks exercise local contracts. Authorised certified mainnet acceptance is
still pending.

The preceding build's installation archive passed in a disposable, network-isolated
Ubuntu 24.04 QEMU guest with systemd 255 and an emulated Nehalem CPU.
Synthetic settings were used. Preflight left the service and account absent.
Group-writable input and a symlinked installation directory failed before
installation. Fresh installation was disabled and inactive with the intended
ownership and modes. The unit ran the executable and held the local lock.
An active update and a changed start block were rejected. A stopped update
preserved the protected settings and checkpoint byte for byte, including
after the service restarted. The worker logged a graceful stop. This rebuild
reran the packaged installer preflight and archive tests.

The archive contains four regular files, comprising the executable, installer,
unit, and build record. Archive and input checksums, the worker dependency
audit, Bash and Node syntax checks, repository static check, source manifest
and Git whitespace checks were run. No private configuration or state was
used for testing. No provider host or Nibrun app was created or tested.
The Rust/frontend/PocketIC suites and canonical Wasm builds were outside the
scope of the worker-only changes.

## Local resources

Both runs used Ubuntu 24.04 containers with one CPU, a 256 MiB cgroup memory
limit, no extra swap and no external network. The release executable ran
with HTTP enabled and made five attempts against a closed loopback HTTPS
endpoint over a 16-second interval. Linux `/proc` reported a peak RSS of
62,064 KiB and 0.32 seconds of process CPU. It exited on SIGTERM.

Earlier successful polling was measured with a separate compiled harness. It used the
production poll loop, SDK actor/identity construction, ledger Candid types,
scan logic and file store. It supplied bounded decoded fixture pages in place
of live gateway responses. The idle case decoded ten empty replies. Catch-up
decoded 32,000 blocks across eight passes, using four pages of up to 1,000
blocks per pass with 75% transfers, and atomically persisted each completed
prefix. These figures omit live transport, certification and funded-candidate
discovery costs.

| Fixture | Elapsed | Process CPU | Peak RSS |
| --- | ---: | ---: | ---: |
| Ten idle passes | 9.09 s | 0.32 s | 57,728 KiB |
| 32,000-block catch-up | 7.86 s | 2.41 s | 100,100 KiB |

Harness RSS and CPU came from `process.resourceUsage()` and
`process.cpuUsage()`. These are local process measurements. Hosted timing,
guest-level memory headroom and live catch-up depend on the actual workload
and provider.

To rerun the successful-poll fixtures, use the build guide's `BUN` and
`TMPDIR` from the worker directory:

```bash
env -i PATH="$PATH" HOME="$TMPDIR" TMPDIR="$TMPDIR" "$BUN" build --no-env-file \
  --compile --target=bun-linux-x64 --no-compile-autoload-dotenv \
  --no-compile-autoload-bunfig test/runtime-harness.mjs --outfile .local/runtime-harness
docker run --rm --network none --memory=256m --memory-swap=256m --cpus=1 \
  --read-only --tmpfs /data:mode=0700 -v "$PWD/.local/runtime-harness:/fixture:ro" \
  ubuntu:24.04 /bin/bash -ec '/fixture idle /data/idle.json; /fixture catch-up /data/catch-up.json'
```

## Hosted acceptance

1. Choose two independent installations, such as one VPS and one Nibrun app.
   Assign different prepared identities and keep their original start and
   separate checkpoints. Stop a moved worker before using its identity and
   state on the destination.
2. Stage private settings and existing state through approved channels.
   Review and apply the existing backend caller-authorisation procedure
   separately. A third simultaneous installation needs its own authorised
   identity.
3. Have the operator set always-on for the specific Nibrun app and confirm
   `"activation": "always"` with `nib apps status --app APP_NAME --json`.
4. Record checkpoints before and after an interval of at least 15 minutes
   without incoming application HTTP requests. Confirm the Nibrun worker kept
   processing beyond five minutes using logs and state. Sample status after
   the quiet period. A TCP health check alone only establishes connectivity.
5. Restart the workers and redeploy the same Nibrun app. Confirm preserved
   identity, original start, checkpoint and Nibrun activation.
6. Measure successful pass intervals, ledger lag, CPU and memory under steady
   polling and catch-up. Record the measurement window and the network
   conditions. Use status, journal and checkpoint evidence.
7. Test one explicitly authorised signed wake from each identity on the trusted
   deployment computer using `node scripts/cli.mjs poke --settings PATH`.
   Check certified acknowledgement and reconcile the backend's response and
   subsequent progress through the existing operational evidence.
8. Stop one installation and confirm the other advances independently. Restore
   the stopped worker and confirm catch-up from its own checkpoint.

See [VPS setup](VPS_SETUP.md) and [Nibrun setup](NIBRUN_SETUP.md).
