# Permanent aliases, subject subscriptions and unified notifications

This release retains the completed alias, admission, indexed subscription,
matching, frontend and upgrade implementation. No mainnet operation is authorized
or performed. The normative contract is [SPEC.md](../../SPEC.md).

## Architectural decision

There are no users or admitted subscribers requiring callback compatibility.
The decoder-budget blocker is eliminated by intentionally replacing the old
callback with one required subject-aware notification record. Every match,
including self, carries its subject. Global activity is an independent explicit
boolean; false plus no matches is never sent. There are no compatibility
receivers, optional extra arguments, alternate dispatch methods, probes,
acknowledgements or retries. The 256-specific-match bound is unchanged.

The new separately compiled receiver uses the pinned `ic-cdk` 0.20.1 ordinary
update macro. Its actual generated decoder retains the default 10,000 skipping
quota; no custom decoder or budget adjustment is used. Maximum-count messages
are decoded through real Wasm entry points in PocketIC. A separate test-only
update returns a SHA-256 of the complete decoded/re-encoded value, so the
largest payload is verified without the lower query serialization budget. This does not claim
compatibility with Jupiter's presently deployed receiver ABI.

## Final callback Candid

```candid
type PokeTarget = variant {
  subaccount : nat64;
  neuron_nonce : nat64;
};
type PokeMatch = record {
  subject : principal;
  target : PokeTarget;
  first_block_index : nat64;
  max_amount : nat;
};
type PokeNotification = record {
  global_activity : bool;
  matches : vec PokeMatch;
};
service : {
  poke : (PokeNotification) -> ();
}
```

`global_activity` reports a qualifying global subscription match during the
completed poll. The vector independently reports specific matches across every
subject, with either boolean value. Empty matches never imply global activity.
Deterministic ordering is subject principal then target kind/number. Each
retained identity independently preserves its minimum qualifying block index
and maximum individual qualifying Nat. There is one attempted call per
subscriber per completed poll, globally matched subscribers first, each phase
in principal order. Actual encoded call cost protects the reserve; failures
continue to later subscribers. Shared-account watchers have no arbitrary cap.

## Production backend Candid

The backend's application interface is retained, including alias-aware preview:

```candid
type SubscriptionPreview = record { subscriber : principal; subject : opt principal; scope : variant { global; subaccount; neuron_nonce }; start : opt nat64; end : opt nat64; target_count : nat32; minimum_units : opt nat; alias_revision : opt blob; aliases_refreshed_at : opt nat64 };
type Price = record { account_icp : nat64; range_icp : nat64; global_icp : nat64 };
type Pricing = record {
  initialized : bool; current : Price; current_effective_at : nat64;
  next : opt Price; next_effective_at : nat64; next_freeze_at : nat64;
  observed_floor_xdr_permyriad : nat64; floor_observed_at : nat64;
  latest_xdr_permyriad : nat64; latest_observed_at : nat64;
  next_carried_forward_due_to_stale_rate : bool;
};
type ObservedLedgerProfile = record {
  symbol : text; decimals : nat8; supports_icrc2_transfer_from : bool;
  neuron_governance : opt principal;
};
type InstanceInfo = record {
  observed_ledger : principal; observed_profile : opt ObservedLedgerProfile;
  sns_root : opt principal;
  icp_ledger : principal; cmc : principal; jupiter_faucet : principal;
  jupiter_historian : principal; surplus_canister : opt principal;
};
type InitArgs = record {
  observed_ledger : principal;
  sns_root : opt principal;
  surplus_canister : opt principal;
};
service : (InitArgs) -> {
  get_instance : () -> (InstanceInfo) query;
  get_pricing : () -> (Pricing) query;
  preview_subscription : (text) -> (variant { Ok : SubscriptionPreview; Err : text }) query;
}
```

## Identity, funding and storage

Only E is currently published. X is retired; A/B below are hypothetical future
identity registrations. Examples `E.A@B.7` and `E.B@A.7` reverse receiver and
subject. Omitting the subject selects self, and `A@A.7`, alias/principal spellings,
and two aliases naming the same principal share semantic identity. Original
memo bytes continue selecting distinct exact Historian funding routes; no
endowment pooling or rewriting occurs. Equality/threshold merging applies only
to `(subscriber, subject, target kind, target number)`.

Aliases resolve at admission to stored principals. Refresh is a replicated call
to trusted Historian, with no browser lookup or matching-time query. A guarded
lane attempts promptly after install and once per UTC day, persisting last
attempt before awaiting. Each page checks actual call cost plus reserve. Bounds
are 128 entries/page, 32 pages, 4,096 entries and 64 KiB encoded response/page.
All pages need one 32-byte revision; strict ordering, progress, grammar,
principal class, complete digest and additive preservation are checked before
publishing. Failure retains the previous accepted table and waits until the
next scheduled opportunity. Timers restore after upgrade from stable metadata.
Unknown aliases consume their admission opportunity without later replay;
another qualifying Faucet payout is needed. Direct principals remain usable.

The E-only golden digest is
`89d9f25601962fb394ad9ebf34aaceccf46b833b9cfd3ed2dc7eb942e9d63625`.
The mock's generated alias-list Candid is checked against the actual subset
contract, and PocketIC exercises replicated replies, multi-page snapshots,
decode/transport failures, inconsistent revisions, bad cursors, malformed
entries, oversized replies, reassignment/removal and additive refreshes.

| Stable regions | Meaning |
| --- | --- |
| 0–8 | Existing keys/value layouts unchanged, including ID 1 self subscriptions |
| 9 | Permanent alias → principal map |
| 10 | Accepted revision, last successful refresh and last attempt |
| 11 | Individually indexed external subscribing relations |

ID 1 remains exactly `Subscription { subscriber; target; minimum_units }`, with
implicit subject=subscriber. ID 11's bounded binary composite key uses a
protocol-qualified destination-account prefix, then length-prefixed subscriber
and subject principals and target kind/u64. Account-prefix iteration reads only
that destination's watchers. There is no wholesale scan, serialized watcher
vector, migration, destructive reset or reindexing of existing self records.

Each transfer independently evaluates each relation's threshold. Ordinary
accounts derive from the subject; neuron controller hashing uses the subject,
with NNS or verified SNS Governance as owner. Ranges expand only at admission.
Shared-page overlays preserve all watchers and publish together with the cursor
without an intervening await. Observation precedes admission for the current
block, so even a subscription watching Event Horizon's own default account
cannot match the Faucet payout that admits it.

Specific aggregation uses `(subject, target)` under each receiver. The earliest
qualifying observed block and maximum individual qualifying Nat amount aggregate
independently across transactions/pages and may describe different transactions.
An earlier below-threshold transfer never supplies the first index. Prior poll
identities are retained in page-local accumulation so overflow on a later page
cannot suppress updates to an already-retained identity. These aggregates and
page-local decoded watcher caches are transient, with no transaction journal,
additional Ledger reads, archive traversal or Index dependency.

## Upgrade evidence

The independently built pre-change debug Wasm comes from the exact reviewed
44d83c0 source archive, whose source manifest was verified. Its digest is
`093a43a8cb52d560829654819b5c4bad42cc955b7c34bafa3f734c416f4643b0`;
[fixture provenance](../../tests/fixtures/event_horizon_44d83c0_debug.provenance.txt)
records the locked toolchain/build command. PocketIC verifies the digest before
installing it, establishes real self/global subscriptions with differing
thresholds, pricing, cursors and pending CMC financial state, then upgrades in
place to the new Wasm with argument-free lifecycle handling. It compares old
subscription/pricing encodings and retained financial/cursor state, adds aliases
and external relations, upgrades again and verifies coexistence and the external
threshold. This is a genuine old-to-new upgrade, separate from new-to-new cases.

## Validation and artifact evidence

Canonical Docker artifacts currently have these identities; final gate results
and resource measurements follow when validation completes. The earlier
intermediate callback candidates are superseded.

| Uncompressed artifact | SHA-256 |
| --- | --- |
| Backend | `2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528` |
| Frontend | `338565e50d946d09201be87131d9c7f5319923320bf68926a17a4f6d19582067` |

The frontend registry and rebuilt bundle pin the backend hash above. Both
production Wasms pass their export audits. Two independent clean Docker builds
and the complete final test suite remain required release gates.

## Coordinated deployment dependencies

Jupiter Faucet and Disburser must implement the exact callback Candid above in
their separate repository. Authenticate Event Horizon, select matches whose
subject equals the receiving canister's own principal, and preserve the existing
local target and threshold filters. Disburser-to-Historian forwarding is a
separate mechanism and is not automatically changed. No Jupiter source is
modified and no compatibility branch is added to Event Horizon.

Operators must separately verify Historian's alias API, additive E-only alias
publication and payout routing. X is retired. Update the incompatible Jupiter
receivers before relying on Event Horizon notifications. Upgrade Event Horizon
in place, verify preserved financial state/cursors, and deploy the reviewed
frontend with its matching backend hash. Mainnet deployments, controller changes,
funding transfers and domain actions require separate authorization. None was
performed by this task.

Primary guidance consulted: current DFINITY
[multi-canister](https://github.com/dfinity/icskills/blob/main/skills/multi-canister/SKILL.md),
[stable-memory](https://github.com/dfinity/icskills/blob/main/skills/stable-memory/SKILL.md),
[canister-security](https://github.com/dfinity/icskills/blob/main/skills/canister-security/SKILL.md),
[cycles-management](https://github.com/dfinity/icskills/blob/main/skills/cycles-management/SKILL.md),
[ICRC Ledger](https://github.com/dfinity/icskills/blob/main/skills/icrc-ledger/SKILL.md)
and [static-site](https://github.com/dfinity/icskills/blob/main/skills/static-site/SKILL.md)
guidance; pinned CDK macro source is checked locally for actual decoder defaults.
