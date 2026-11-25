# Continuous Event Horizon ICP worker

Use the same Linux x86-64 executable on Webdock, netcup, and Nibrun with
manually enabled always-on activation. The worker watches the ICP ledger and
sends a signed zero-argument wake to canonical Event Horizon when existing
matching rules identify potentially relevant activity. See [Start here](START_HERE.md).

## Behavior and limits

The normal loop targets one second from the beginning of one pass to the next.
Passes never overlap. If a pass takes longer than one second, the next begins
when it finishes. Repeated failures use bounded backoff. A one-second target is
not a guarantee of one-second transaction-to-notification latency.

Each pass reads a contiguous prefix from its saved ledger cursor, captures an
ending boundary, and follows only ledger-advertised archive callbacks. It never
advances across an unread gap. Default bounds are four pages of up to 1,000 blocks
and ten seconds of scanning checked between completed blocks. HTTP, signed-poke
acknowledgment and the whole pass have separate deadlines.

Authentic Faucet transfers to the backend request a wake even when the evidence
does not establish a candidate. A transport failure during discovery retries
the uncommitted prefix before sending a wake. Candidate discovery uses the
original ICRC-1 memo, existing `preview_subscription`, current pricing, and the
Historian exact RawIcp route summary. Funding evidence must be complete and
sufficient. Ordinary matching checks incoming transfer destination and raw amount;
a learned global interest considers any new block. Matching precedes learning
within each block. Accounts collapse to their lowest learned threshold, since the
worker only needs to decide whether the backend should wake.

At most one signed poke is sent per pass. The checkpoint is saved after its
certified acknowledgment. Failed or ambiguous pokes leave the cursor unchanged
for retry. A successful poke followed by a storage failure may produce a duplicate
wake. Independent workers may also wake the backend for the same event. The backend's
existing coalescing and reserve protection still apply.

### Candidate interests are not authoritative subscriptions

The backend has no admitted-subscription enumeration query. Therefore the worker
cannot mirror its authoritative subscription set exactly. Current alias, price,
or Historian evidence may differ from the backend's earlier admission decision.
This can produce an unnecessary wake or delay acceleration until a later Faucet
payout. Transport failures retry the same prefix; an ineligible or incomplete
structured evidence response is not a subscription and waits for a later payout.
On-chain fallback polling remains the correctness and recovery path.
The prospective start also means existing subscriptions are only learned from
later qualifying payouts, and the checkpoint's 8 MiB size limit stops progress
if the candidate map grows beyond it.

A zero-argument poke carries no recipient, transaction, or authoritative cursor.
The backend still reads all unread blocks from its own cursor and repeats its
admission and matching work before sending notifications. This provides earlier
reactions without frequent on-chain checks for new relevant activity; it does not
eliminate backend per-block processing or guarantee lower cycles than today's
slow polling. Pricing, ICP-to-cycles conversion and other backend lanes remain.

## Build and test

Run on the trusted Linux deployment computer with Node.js 24, npm, Python
3, Bash, Git and standard Linux tools. This example installs the pinned Bun
compiler into an ignored directory. A separately installed Bun 1.4.5 also
works by setting `BUN` to its path.

```bash
cd workers/icp-poke
umask 077
mkdir -p .local/toolchain .local/tmp
export TMPDIR="$PWD/.local/tmp"
npm install --prefix .local/toolchain --ignore-scripts --no-audit --no-fund --save-exact @oven/bun-linux-x64@1.4.5
export BUN="$PWD/.local/toolchain/node_modules/@oven/bun-linux-x64/bin/bun"
npm ci --ignore-scripts --no-audit --no-fund
npm test
npm run package
npm run test:artifact
npm audit --omit=dev --audit-level=high
(cd dist && sha256sum -c SHA256SUMS)
```

`dist/eh-poke-linux-x86_64` is the executable for all three hosts.
`dist/eh-poke-vps-linux-x86_64.tar.gz` contains the identical executable, VPS
installer, systemd unit and `BUILD.txt`. The latter identifies the source
revision, local change status, build inputs and toolchain. `SHA256SUMS` covers
the executable, archive and build record. The package script stages only
explicit inputs and installs the locked dependencies in a clean build
environment. See [validation](VALIDATION.md) for how this build was tested.

The executable targets Linux x86-64 with SSE4.2 and glibc 2.17 or newer.
Use a current Linux kernel and ordinary OS HTTPS trust support on
VPSs. The binary embeds Bun and the JavaScript dependencies. [Bun installation and CPU requirements](https://bun.com/docs/installation)

## Settings and state

| Worker value | Meaning |
| --- | --- |
| `EH_IDENTITY_SEED` | Private dedicated Ed25519 seed, canonical base64 |
| `EH_START_BLOCK` | Original ledger boundary as decimal text |
| `EH_BACKEND_CANISTER` | Canonical backend, default `eo6ei-gaaaa-aaaar-qchra-cai` |
| `EH_ICP_HOST` | HTTPS origin, default `https://icp-api.io` |
| `EH_WORKER_NAME` | Non-sensitive label for logs |
| `EH_PAGE_SIZE` | Default 1,000, maximum 2,000 |
| `EH_MAX_PAGES` | Default 4, maximum 32 |
| `EH_CHECKPOINT_PATH` | Optional absolute file path on persistent storage |

Use `--settings PATH` for a protected JSON file with string values. This file
is the sole source of worker values when selected. Otherwise the service reads
`EH_*` values from its environment. Checkpoint precedence is
`--checkpoint PATH`, then the selected source's `EH_CHECKPOINT_PATH`, then
`NIBRUN_DATA_DIR/checkpoint.json`. When supplied, `NIBRUN_DATA_DIR` must be an
existing absolute directory. The checkpoint must stay within it. The service
rejects symlinks at that directory and along the checkpoint path below it.
On a VPS, choose an absolute checkpoint file path on persistent storage.
Retain the checkpoint for that identity.

HTTP is off by default. `--http-port PORT_NUMBER` overrides
`NIBRUN_HTTP_PORT`. The ordinary `PORT` environment variable has no effect.
When enabled, the listener binds `0.0.0.0`. `GET /healthz` and
`HEAD /healthz` return HTTP 200 after a successful pass and 503 during
startup or retry. The JSON contains the worker version, lifecycle state, last
successful pass time, and, after success, the next block and lag as decimal
strings from the last completed pass. On stop the listener closes immediately
and the active bounded pass can finish. Per-pass acknowledgement and persistence
rules still apply if the host terminates it first. A socket or HTTP 200 alone
cannot establish ledger catch-up. Check the freshness of the last success, lag
and journal/checkpoint evidence.

VPS systemd supplies `/etc/event-horizon/worker.settings.json` and
`/var/lib/event-horizon-worker/checkpoint.json`. That protected settings file is
`root:ehworker` mode 0640. Nibrun supplies `NIBRUN_HTTP_PORT` and
`NIBRUN_DATA_DIR`, normally `/app/data`. A VPS holds a local systemd `flock`.
On Nibrun run one process for the app. Each independent installation needs its
own identity and checkpoint. A stale-edit digest detects local edits. The write
contract relies on a single writer. A missing checkpoint replays from the
original start. Corrupt or mismatched state prevents progress until repaired.

The ICP client uses its configured mainnet trust root, verifies query
signatures and certified update acknowledgements, and validates the backend's
protocol anchors. Its trust settings and canister interfaces are unchanged.

### Offline checks and source development

```bash
./dist/eh-poke-linux-x86_64 --validate --settings /private/worker.settings.json
npm start -- --settings /private/worker.settings.json --checkpoint /persistent/checkpoint.json
```

`--validate` exits after offline validation. If supplied explicitly with
`--checkpoint PATH`, it also validates any existing checkpoint without changing
it. The source launch command starts real polling. Use it only for a worker
and state directory you are authorised to run. Use
`node scripts/cli.mjs help` for the separate public-query `check` and
`dry-run` diagnostics and the identity/caller preparation utilities.

- [VPS setup](VPS_SETUP.md)
- [Nibrun setup](NIBRUN_SETUP.md)
- [Worker permission and customer subscriptions](SUBSCRIPTIONS.md)
- [Local validation and hosted acceptance](VALIDATION.md)
