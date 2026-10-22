# Permanent aliases, subject subscriptions and unified notifications

This release retains the completed alias, admission, indexed subscription,
matching, frontend and upgrade implementation. No mainnet operation is authorized
or performed. The normative contract is [SPEC.md](../../SPEC.md).

## Validation status

Validated on 2026-10-22 UTC. The complete managed run passed 133 Rust tests,
138 frontend tests and all 118 isolated PocketIC scenarios. Formatting, Clippy
with warnings denied, static/source checks, dependency security scans and both
production export audits passed. Two independent, uncached Docker builds from
`e1177c946248a9e4c652849b100f82d75ff9bad5` produced byte-identical complete
artifact sets. The final release-record/source-manifest update changes no runtime
or build inputs; the canonical rebuild must retain the artifact hashes below.
Deployment still requires the coordinated Jupiter receiver update described below.

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

## Reviewed source context

The retained upgrade baseline is Event Horizon source
`44d83c04a190dba2b8420c0d812e1fd8ee07ff19`. The prior alias implementation review
recorded Jupiter `1c86d9b32c774a470489b4e4a5cfe4ddfaae7341` for Historian's alias
API and E-only publication. That earlier source review neither verifies live
installation nor implements the new receiving ABI. This continuation modifies
only Event Horizon. Its complete code/test validation and independent builds
use `e1177c946248a9e4c652849b100f82d75ff9bad5`; the subsequent release record and
source manifest contain evidence only.

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

The earlier intermediate callback candidates are superseded.

| Uncompressed canonical artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Backend | 1,428,428 | `2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528` |
| Frontend | 3,757,773 | `338565e50d946d09201be87131d9c7f5319923320bf68926a17a4f6d19582067` |

The frontend registry, rebuilt bundle and canonical frontend Wasm contain the
reviewed backend hash above. Python SHA-256 recomputation independently agrees
with the canonical artifact manifest. Both production export audits pass; the
backend has 11 total Wasm exports and only its approved read-only application
queries. The production-query PocketIC scenario also passes. Canonical build
metadata records Rust/Cargo 1.94.1, ic-wasm 0.9.7, Node 18.20.4, npm 9.2.0 and
fixed `SOURCE_DATE_EPOCH=1700000000`.

| Gate | Result |
| --- | --- |
| `cargo run -p xtask -- test_all` | Passed: 133 Rust, 138 frontend, 118 PocketIC; zero failures |
| Static validation, source manifest, canonical-context fixture | Passed in the managed run |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed |
| `./tools/scripts/security-scan` | Passed: cargo-audit, cargo-deny, npm audit, OSV |
| `npm run verify:reproducible-artifacts` | Passed: two fresh BuildKit builders, no build cache; all artifact files identical |
| Canonical Docker artifacts and production export audits | Passed; hashes above |

Security retains exactly the four existing documented exceptions:
RUSTSEC-2025-0012, RUSTSEC-2024-0384, RUSTSEC-2024-0436 and
RUSTSEC-2021-0127. No dependency exception was added. npm audit reported zero
vulnerabilities. Unit-test discovery's 118 ignored PocketIC tests were all
executed separately by the managed integration runner; none was skipped from
release validation. Raw local logs are retained under
`observations/unified-notifications/`, outside canonical source artifacts.

## Notification regression evidence

All scenarios below pass in the complete PocketIC run. The receiving crate
independently defines the Candid types and compiles its own Wasm.

| Required behavior | Evidence |
| --- | --- |
| Global-only: true flag, no matches | `shared_icp_unified_notifications_cover_global_specific_and_coalescing` |
| Self-only: false flag, explicit self subject | `generic_icrc3_specific_activity_executes_both_receivers` |
| External-only and mixed self/external | `cross_subject_independent_thresholds_exact_routes_and_callbacks` |
| Global plus specific matches in one call | Shared ICP unified scenario and cross-subject global/external phase |
| Same target numbers under multiple subjects | Cross-subject scenario retains A.7, B.7 and C.7 separately |
| Independent thresholds, first indices and maxima | Cross-subject scenario; `first_block_index_spans_pages_thresholds_namespaces_and_upgrade` |
| 256 combined hints and deterministic first-retained order | `external_ranges_pages_archives_neurons_and_combined_payload_bound`, on ICP, ICRC-3 and SNS |
| Large Nat and full-width nat64 fields | `generic_nat_amount_survives_end_to_end_poke`; maximum receiver scenario |
| Actual CDK decoding in separate receiver | `unified_receiver_decodes_maximum_notifications_with_pinned_cdk_defaults` |
| One attempted call despite mixed activity | Sender attempt counters and fresh receiver state in combined-payload scenario; unified dispatch unit checks |
| Reserve protection and failure continuation | `global_fanout_reserve_skips_still_attempt_all_recipients`; missing/trapping external receivers; trapping global receiver followed by specific delivery |
| No false/empty notification | Dispatch unit regression; no-work and admission-timing PocketIC cases |
| Real old-to-new financial/operational upgrade | `pre_alias_layout_upgrades_without_rewriting_financial_or_subscription_state` |

The old-to-new scenario passed using the checksum-verified historical Wasm
listed above. Its source test compares original subscription/pricing encodings,
cursors and pending CMC state, then checks additive aliases/external relations
and a further upgrade. The other 117 scenarios retain the existing reader,
pricing, funding, scheduler, frontend certification and upgrade coverage.

## Resource measurements

These are local PocketIC measurements, not mainnet cost forecasts. Shared-account
work remains proportional to interested relations; no arbitrary watcher limit,
new price, dynamic polling rule or subscriber-accounting mechanism was added.

The real poll/one-way callback/receiving-canister path retained 256 combined
self/external identities and attempted exactly one call in each ledger mode:

| Mode | Payload bytes | Event Horizon cycles consumed |
| --- | ---: | ---: |
| ICP | 8,511 | 170,252,429 |
| ICRC-3 | 156,921 | 1,249,570,151 |
| Verified SNS | 156,921 | 1,250,190,823 |

The ICRC-3/SNS cases use 4,097-bit amounts; ICP uses its actual bounded ledger
amount type. Measurements include poll/aggregation/dispatch and ten settling
ticks, with ordinary framework limits and no decoder override.

Separately compiled receiver stress cases use 256 distinct 29-byte subjects,
full-width targets and block indices, with both global flag values:

| Nat bits | Payload bytes | Receiver cycles, false flag | Receiver cycles, true flag |
| ---: | ---: | ---: | ---: |
| 257 | 21,826 | 66,256,494 | 66,690,450 |
| 4,097 | 162,370 | 439,442,204 | 439,749,828 |
| 32,769 | 1,210,946 | 3,221,827,774 | 3,221,934,170 |

Receiver cycle samples cover decoding and storage, excluding the subsequent
test-only digest verification. The pinned CDK's generated entry point uses its
unchanged 10,000 skipping quota. The digest covers the entire decoded/re-encoded
notification under the ordinary update instruction budget; it avoids serializing
the largest stored value under the lower query budget. Neither the decoder
configuration nor the 256-match protocol limit was weakened. These bit sizes
are stress fixtures, not newly imposed amount limits.

For 1,024 independently stored watchers of one shared account, with thresholds
0–15 and transfer amount 20, the settled measurements (30 post-poll ticks) were:

| Transfers to the shared account | ICP cycles | ICRC-3 cycles |
| ---: | ---: | ---: |
| 1 | 5,958,018,362 | 5,962,511,174 |
| 32 | 6,186,080,409 | 6,194,336,592 |
| 256 | 7,834,486,405 | 7,866,189,640 |

All 1,024 watchers received an attempted callback in each completed poll.
These synthetic recipients measure sender work, not remote execution. Stable
memory remained 100,728,832 bytes before/after seeding; records fit within the
already allocated stable buckets. Existing regions 0–8 and their layouts are
unchanged; the three additive regions retain their initial 24 MiB bucket cost.

Global fan-out measured 1,451,228,234 cycles for 256 recipients and
5,742,188,989 for 1,024. Both cases recorded exactly the intended attempted and
accepted one-way sends, with stable memory unchanged at 100,728,832 bytes.
Accepted sends do not establish receiver execution. No test claims remote
cross-canister execution ordering. Exact instruction counts are not emitted by
the capacity fixture; completion is checked under ordinary execution limits.

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
