# Event Horizon backend canister

This canister observes one immutable configured ledger, admits Jupiter Faucet-funded subscriber declarations, and sends best-effort one-way wake-up hints when qualifying transfers appear. Subscribers retain authoritative cursors and reconciliation responsibility.

## Mainnet identity

- CLI name: `event_horizon`.
- Canonical ICP backend: `eo6ei-gaaaa-aaaar-qchra-cai`.
- Companion frontend: `event_horizon_frontend`, `ej7c4-lyaaa-aaaar-qchrq-cai`;
  see its [README](../frontend/README.md).

These are the existing mainnet canisters. Local replicas and additional Ledger
instances have separate IDs. From the repository root, check name resolution
without deploying:

```bash
icp canister status event_horizon -e ic --id-only
```

The expected result is `eo6ei-gaaaa-aaaar-qchra-cai`. `icp.yaml` declares the
name; the ignored operator-local `.icp/data/mappings/ic.ids.json` binds it to
the mainnet principal. A fresh checkout must restore that mapping. See
[canister name resolution](../../docs/operations/deployment.md#canister-name-resolution)
before any deployment. Documented identity does not establish the live module
hash or controller state; verify those independently.

## Immutable configuration and ledger modes

Installation persists `observed_ledger`, optional `sns_root`, and optional `surplus_canister` exactly once. The latter is an independent canister principal whose default all-zero-subaccount ICP AccountIdentifier receives eligible surplus ICP. `null` disables diversion. Ordinary upgrades take no configuration argument, and production exposes no setter or administrator method. The ICP Ledger, CMC, NNS Governance, Jupiter Historian, and Jupiter Faucet remain Wasm-level protocol anchors, so every instance uses the same reviewed backend bytes.

The canonical ICP instance reads the legacy ICP `query_blocks` log for both admission and observation and uses fixed NNS Governance ownership for neuron targets. Generic instances retain ICP as the admission and funding asset while reading a compatible ICRC-1/ICRC-3 observed ledger. A generic SNS instance enables neuron targets only after verifying the configured SNS Root's Root/Ledger/Governance tuple and the Ledger's Governance-default minting account. No Index canister is required.

## Admission and notification

Jupiter Faucet payments and exact Historian route evidence admit permanent global, numeric-subaccount, inclusive subaccount-range, neuron-nonce, or neuron-range declarations. Ranges expand into the existing watched-account map and contain at most 256 targets. Thresholds use the observed token's decimals; account, range, and global prices remain denominated and funded in ICP.

Initial reference bases are 10 ICP for an account, 20 ICP for a range, and
1,000 ICP for a global subscription. They remain subject to the existing
monthly CMC-rate formula and independent upward rounding. The stored `Price`
shape is unchanged; range remains exactly derived from the 20 ICP basis using
`ceil(global_icp / 50)`.

The backend can be installed, initialize its observed-ledger profile, and
observe the configured ledger without a Jupiter Faucet alias. A reviewed alias
is required before opening the instance for subscriber use: Faucet payout plus
Historian route evidence is how subscriber declarations are funded and
admitted.

Qualifying activity is coalesced once per subscriber per poll. A specific callback
reports sorted subject-qualified target/`first_block_index`/maximum-raw-amount entries.
Self entries remain in the first vector; external entries use the optional second
vector, or `poke_subjects` for external-only activity. `poke([], null)` represents
global-only activity. The minimum index and maximum amount may come from different
transactions. Direct-Ledger readers use their own cursors and bounded archive-aware
reconciliation; see the [subscriber guide](../../docs/subscriber-guide.md#direct-ledger-reconciliation).
Specific matches take precedence when both match, so an
intentionally global consumer treats either vector shape as a wake-up.

Within each completed poll, the backend attempts all actually globally matched
recipients first, including mixed matches with specific payloads, then
specific-only recipients. Each phase uses ascending principal order regardless
of endowment amount. This fixed rule adds no configurable priority tiers or
preferential cadence. Failed or unaffordable sends do not block later attempts;
per-call reserve checks remain in force. Outbound attempt order guarantees
neither delivery nor remote execution order between different canisters, Index
catch-up, or ordering across independent instances. No acknowledgement or
completion response is awaited, and there is no durable notification queue.
Event Horizon never discovers, queries, inspects or waits for an Index.
Index-dependent subscribers should consider a bounded, coalesced follow-up
approximately five seconds after their initial reconciliation; see
[Index catch-up](../../docs/subscriber-guide.md#index-catch-up).

Any patron may fund account, range, neuron or global declarations without
subscriber consent, including irrelevant targets or weaker thresholds. Scoped
subscribers should normally implement `poke(vec PokeMatch)`, authenticate the
caller, and check their own per-Ledger target/amount policy before expensive
work. Trusted origin does not establish local interest. Zero-argument `poke()`
remains compatible with the same outgoing payload through Candid; it is suitable
only for deliberately accepting all authenticated wakes with cheap, bounded and
coalesced processing. See the [subscriber guide](../../docs/subscriber-guide.md)
for the recommended handler and local global-interest policy.

The callback remains one-way and best-effort. Event Horizon does not negotiate
signatures, retry or account for delivery. Subscribers retain independent
authoritative Ledger reconciliation and periodic timers.

Productive endowment principal remains in Jupiter's staking path. Event Horizon
receives recurring maturity-derived raw-ICP payouts, pools them, and converts
retained ICP through the CMC while preserving reserve protection,
duplicate-safe transfer identity, and upgrade recovery. A configured recipient
begins at diversion level zero; it does not bypass the seven-day policy or
either 150 T gate. Before value moves, a split freezes the destination account,
memo, amount, and fee. Recovery continues with that identity rather than
re-reading instance configuration. Canonical ICP currently installs with no
recipient. This changes no production API.

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

The production application surface contains only the read-only `get_instance`, `get_pricing`, and `preview_subscription` queries. Debug/test methods are built only with the test feature and are absent from canonical production Wasm.

Stable state holds immutable instance/profile data, reader cursors, subscriptions, pricing history, and funding recovery state. Schema evolution is additive and covered by upgrade scenarios. The canonical production Wasm deliberately remains separate from debug/test Wasms.

## Component commands

```bash
cargo test --locked -p event-horizon --lib
./tools/scripts/build-release
./tools/scripts/docker-build
```

The host build is for local inspection; only the Docker build produces canonical release evidence.

The complete protocol is normative in [`SPEC.md`](../../SPEC.md). See the [subscriber guide](../../docs/subscriber-guide.md), [architecture](../../docs/architecture/overview.md), [production surface](../../docs/architecture/production-surface.md), and [deployment](../../docs/operations/deployment.md).

Permanent Historian identity aliases and explicit `subscriber@subject` declarations
are supported. Omitted subject means self. The canonical ICP payment prefix is
`E.`; retired `X` is not offered. Identity aliases are not general payout routes.
The builder shows resolved principals and validates alias declarations through
the selected backend's local `preview_subscription` query. See the
[subscriber guide](../../docs/subscriber-guide.md) for exact-route funding, unknown-cache admission, external-only callbacks,
local subject filtering, and authoritative reconciliation.
