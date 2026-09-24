# Adaptive surplus diversion validation

## Identity and scope

- Starting commit: `304b26e9e6facc6f8812e61c56f98e712e05f803`.
- Branch: `codex/surplus-diversion`.
- Final commit: the handoff `HEAD` containing this report and regenerated source manifest; its exact SHA is reported with the archive because a Git commit cannot contain its own identifier.
- Implementation commit: `7904af8` (`Add adaptive surplus diversion`).
- Final evidence/manifest commit: the handoff `HEAD` reported separately.
- Production surplus destination: `None`; production diversion is disabled and begins with no accumulated entitlement when a destination is later compiled in.
- No mainnet deployment, Jupiter Faucet `X` alias publication, controller change, or controller removal was performed.

## Durable policy and funding state

Two additive stable-memory IDs were introduced without changing IDs 0–6:

| ID | State |
|---:|---|
| 7 | `FundingStateV2`, the single authoritative next money-moving action |
| 8 | `SurplusPolicyState`, containing initialization, epoch start, hourly observed minimum, and level |

On first ID 7 use, `Uninitialized` reads old ID 2 and writes exactly one migrated state:

- old `Idle` → new `Idle`;
- old `TransferPending { amount, fee, created_at_time }` → `CmcTransferPending` with the identical identity and both planned-surplus fields zero;
- old `NotifyPending { block_index }` → `CmcNotifyPending` with the identical block and both planned-surplus fields zero.

After migration, ID 7 is exclusive authority and old ID 2 remains untouched historical storage. The tracked fixture `event_horizon_range_validated_304b26e_debug.wasm` was built at the exact starting commit and has SHA-256 `3706743f88548b7c6c84462fd3a53a4ca327cc182d0117dd8c148eec418cec08`.

## Exact controller policy

The existing hourly funding lane supplies one liquid-cycles observation. Enabling starts a fresh seven-day epoch at level 0. The lowest observed balance in the epoch causes at most one transition at the first opportunity after expiry:

- at least 150 T: `level = min(level + 1, 19)`;
- 100 T inclusive to 150 T exclusive: `level = level.saturating_sub(1)`;
- below 100 T: `level = 0`.

The diversion percentage is `level × 5`, hence 0–95% with no 100% state. A long inactive interval never creates multiple transitions. Disabled destination, invalid level, or backwards-time state fails closed to level zero. Planning and the post-CMC gate both require the current liquid balance to be at least 150 T; the stored level is not immediate transfer authority.

## Exact split and ordering

For balance `B`, Ledger fee `F`, and gated percentage `P`:

- `P = 0`, `B <= 2F`, or a zero split leg uses the existing one-fee CMC plan `B - F`;
- otherwise `allocatable = B - 2F`;
- `surplus = floor(u128(allocatable) × P / 100)`;
- `retained = allocatable - surplus`.

Thus `retained + surplus + 2F = B`, multiplication cannot overflow at `u64` balances, and every rounding remainder stays retained. At 95%, a splittable positive allocation always retains a positive share.

The exact persistent sequence is plan → retained legacy Ledger transfer → CMC `notify_top_up` success → re-check 150 T → surplus legacy Ledger transfer. CMC Processing keeps the same plan; refund or terminal error cancels surplus. New ICP arriving during a pending plan is excluded and remains for the next Idle sweep. The surplus memo is decimal `6004796182999946033`, big-endian ASCII `SURPLUS1`.

## Upgrade and financial recovery results

PocketIC upgraded the exact `304b26e…` fixture after establishing account, range, and global subscriptions, initialized/frozen pricing, Ledger cursor and polling mode, and old CMC states. Old Idle, TransferPending, and NotifyPending each migrated once with zero planned surplus. No phantom transfer appeared, the destination/controller remained disabled at level zero, and all subscription/pricing behavior was preserved.

Current-Wasm upgrades resumed safely from retained CMC transfer pending, CMC notify pending with planned surplus, and surplus transfer pending. Tests also proved retained-first ordering; no surplus during CMC Processing; cancellation on refund, terminal error, and falling cycles; CMC lost-response duplicate recovery; surplus success, lost-response duplicate recovery, clean system reject, insufficient funds, bad fee, future timestamp retry, `TxTooOld`, and pending-transfer upgrade. No test produced two accepted surplus transfers for one persisted identity.

The subscription-independence regression moved raw ICP through retained and surplus funding before the Ledger crawler processed the original Faucet block; Historian validation and admission still succeeded from historical Ledger/source/memo/route evidence.

## Validation results

- `npm ci` — passed.
- `cargo run -p xtask -- check` — passed: static checks, format, Clippy with warnings denied, 46 backend Rust unit tests, 12 frontend tests, and all workspace tests.
- `cargo run -p xtask -- pocketic` — 36/36 passed.
- `cargo run -p xtask -- local-smoke` — 36/36 passed.
- Exact `304b26e…` migration and all new funding/policy upgrade regressions — passed within PocketIC.
- `cargo run -p xtask -- security` — passed: 0 vulnerabilities; four existing explicitly permitted unmaintained transitive warnings; cargo-deny and OSV passed.
- `cargo run -p xtask -- release` — passed, including artifact checksums and production export audits.
- `./tools/scripts/docker-build` — passed; canonical artifact checksums verified.
- `cargo run -p xtask -- repro` — passed: `release artifacts match across two clean builds`.
- `DFX_IDENTITY=codex_local icp build -e local` — passed: `Canisters built successfully`.
- Source-manifest generation/verification — passed after this report and all final documentation were tracked.

The maximum-range regression remained the largest explicitly reported integration operation at `72,004,641` cycles. Stable memory before/after that operation remained `75,563,008` bytes, and total memory remained `78,069,724` bytes in the final PocketIC run. No high-frequency surplus timer or per-poll policy write was added.

## Canonical artifacts and surfaces

- Backend Wasm SHA-256: `751b3a7a5734299a3e66c3b4d461a074db08ad4de6f7aa994e3e401dbab2e184`.
- Frontend Wasm SHA-256: `51991f35fa07152ea9b65f90c562604b5c65a6b968e2a86d5d3e93e904d04857`.
- Backend audit: 9 total Wasm exports; only application method `canister_query get_pricing`.
- Frontend audit: 7 total Wasm exports; only application method `canister_query http_request`; no HTTP update proxy.

## Remaining limitations

The production receiver is intentionally undecided/disabled. Enabling requires changing the compile-time constant to one reviewed principal, rebuilding and revalidating a new canonical Wasm before controller removal. The observed epoch minimum samples only hourly funding opportunities, not continuous balance. Existing best-effort notification, no-archive-traversal, daily-pricing-sample, and narrow extraordinary Ledger ambiguity limitations remain. Event Horizon contains no treasury/governance behavior, transaction-history recovery, administrator repair, destination setter, withdrawal method, or subscriber-specific funding accounting.
