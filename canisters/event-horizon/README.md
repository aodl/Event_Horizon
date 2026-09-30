# Event Horizon backend canister

This canister observes one immutable configured ledger, admits Jupiter Faucet-funded subscriber declarations, and sends best-effort one-way wake-up hints when qualifying transfers appear. Subscribers retain authoritative cursors and reconciliation responsibility.

## Immutable configuration and ledger modes

Installation fixes `observed_ledger` and optional `sns_root` alongside the protocol ICP Ledger, CMC, Historian, Faucet, and optional compiled surplus destination. These values are not administrative settings and cannot be changed after installation.

The canonical ICP instance reads the legacy ICP `query_blocks` log for both admission and observation and uses fixed NNS Governance ownership for neuron targets. Generic instances retain ICP as the admission and funding asset while reading a compatible ICRC-1/ICRC-3 observed ledger. A generic SNS instance enables neuron targets only after verifying the configured SNS Root's Root/Ledger/Governance tuple and the Ledger's Governance-default minting account. No Index canister is required.

## Admission and notification

Jupiter Faucet payments and exact Historian route evidence admit permanent global, numeric-subaccount, inclusive subaccount-range, neuron-nonce, or neuron-range declarations. Ranges expand into the existing watched-account map and contain at most 256 targets. Thresholds use the observed token's decimals; account, range, and global prices remain denominated and funded in ICP.

The backend can be installed, initialize its observed-ledger profile, and
observe the configured ledger without a Jupiter Faucet alias. A reviewed alias
is required before opening the instance for subscriber use: Faucet payout plus
Historian route evidence is how subscriber declarations are funded and
admitted.

Qualifying activity is coalesced once per subscriber per poll. A specific callback reports sorted target/maximum-raw-amount pairs; an empty vector represents global-only activity. The callback is intentionally one-way and best-effort. Event Horizon does not retry or account for delivery, and subscribers must retain their own authoritative ledger cursors.

Funding converts retained ICP through the CMC while preserving reserve protection, duplicate-safe transfer identity, and upgrade recovery. The optional adaptive surplus destination is compiled immutably and is currently disabled in production.

## Production surface and state

The production application surface contains only the read-only `get_instance` and `get_pricing` queries. Debug/test methods are built only with the test feature and are absent from canonical production Wasm.

Stable state holds immutable instance/profile data, reader cursors, subscriptions, pricing history, and funding recovery state. Schema evolution is additive and covered by upgrade scenarios. The canonical production Wasm deliberately remains separate from debug/test Wasms.

## Component commands

```bash
cargo test --locked -p event-horizon --lib
./tools/scripts/build-release
./tools/scripts/docker-build
```

The host build is for local inspection; only the Docker build produces canonical release evidence.

The complete protocol is normative in [`SPEC.md`](../../SPEC.md). See the [subscriber guide](../../docs/subscriber-guide.md), [architecture](../../docs/architecture/overview.md), [production surface](../../docs/architecture/production-surface.md), and [deployment](../../docs/operations/deployment.md).
