# Event Horizon backend canister

This canister observes one immutable configured ledger, admits Jupiter Faucet-funded subscriber declarations, and sends best-effort one-way wake-up hints when qualifying transfers appear. Subscribers retain authoritative cursors and reconciliation responsibility.

## Immutable configuration and ledger modes

Installation persists `observed_ledger`, optional `sns_root`, and optional `surplus_canister` exactly once. The latter is an independent canister principal whose default all-zero-subaccount ICP AccountIdentifier receives eligible surplus ICP. `null` disables diversion. Ordinary upgrades take no configuration argument, and production exposes no setter or administrator method. The ICP Ledger, CMC, NNS Governance, Jupiter Historian, and Jupiter Faucet remain Wasm-level protocol anchors, so every instance uses the same reviewed backend bytes.

The canonical ICP instance reads the legacy ICP `query_blocks` log for both admission and observation and uses fixed NNS Governance ownership for neuron targets. Generic instances retain ICP as the admission and funding asset while reading a compatible ICRC-1/ICRC-3 observed ledger. A generic SNS instance enables neuron targets only after verifying the configured SNS Root's Root/Ledger/Governance tuple and the Ledger's Governance-default minting account. No Index canister is required.

## Admission and notification

Jupiter Faucet payments and exact Historian route evidence admit permanent global, numeric-subaccount, inclusive subaccount-range, neuron-nonce, or neuron-range declarations. Ranges expand into the existing watched-account map and contain at most 256 targets. Thresholds use the observed token's decimals; account, range, and global prices remain denominated and funded in ICP.

The backend can be installed, initialize its observed-ledger profile, and
observe the configured ledger without a Jupiter Faucet alias. A reviewed alias
is required before opening the instance for subscriber use: Faucet payout plus
Historian route evidence is how subscriber declarations are funded and
admitted.

Qualifying activity is coalesced once per subscriber per poll. A specific callback reports sorted target/maximum-raw-amount pairs; an empty vector represents global-only activity. The callback is intentionally one-way and best-effort. Event Horizon does not retry or account for delivery, and subscribers must retain their own authoritative ledger cursors.

Funding converts retained ICP through the CMC while preserving reserve protection, duplicate-safe transfer identity, and upgrade recovery. A configured recipient begins at diversion level zero; it does not bypass the seven-day policy or either 150 T gate. Before value moves, a split freezes the destination account, memo, amount, and fee. Recovery continues with that identity rather than re-reading instance configuration. Canonical ICP currently installs with no recipient.

An instance can be installed, initialize its observed-ledger profile, and observe
the configured Ledger without a Jupiter Faucet alias. A reviewed alias is
required before subscriber declarations can be funded and admitted because
Faucet payout plus Historian route evidence is the admission mechanism.

The production constructor is:

```candid
record {
  observed_ledger : principal;
  sns_root : opt principal;
  surplus_canister : opt principal;
}
```

Installation rejects an anonymous, management-canister, or self surplus
recipient, but imposes no relationship to the observed Ledger, SNS, or Jupiter.
`get_instance` and the initial CONFIG log expose the complete immutable tuple.

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
