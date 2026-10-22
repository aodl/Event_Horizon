# Permanent aliases and subject subscriptions: release record

This release candidate prepares permanent Historian aliases and independent
subscribers watching another canister's accounts. It authorizes no mainnet
operation. The normative contract is [SPEC.md](../../SPEC.md); integration and
security advice remains centralized in the [subscriber guide](../subscriber-guide.md).

## Reviewed sources and artifact identity

Event Horizon upstream was checked against
`44d83c04a190dba2b8420c0d812e1fd8ee07ff19` before implementation and re-fetched
during release preparation; it remained at that revision. Jupiter's actual
Historian API and E-only publication were inspected at prerequisite
`1c86d9b32c774a470489b4e4a5cfe4ddfaae7341`. This source inspection does not verify
that those Jupiter contracts are installed live.

The backend runtime candidate was built canonically from
`d68b5d88199947fc3f3550698f8d0690971eb0f9`. The coordinated frontend pin and bundle
were built from `f02b28e1cb2ef51e4aba47a1a46b417d91b5cc73`. The final release
source is the clean commit containing this record and its source manifest;
canonical commands print its full revision. Later changes in that commit are
regression coverage, audit checks and this record; they must reproduce these
same runtime artifacts:

| Uncompressed canonical artifact | SHA-256 |
| --- | --- |
| Backend | `1e15b0272a1c692ab8c761a8645bbb575eefe69bf5d5a5a48b9356f9be8a707e` |
| Frontend | `8e16de80f439c1e1166906a008b92f0e26d01b8f2c70a8a428fc4e141c716bb5` |

These hashes were obtained from Docker artifacts, verified with the artifact
manifest and independently recomputed with Python's SHA-256. The frontend
registry and rebuilt bundle pin the backend hash above; the pinned string was
also checked in the canonical frontend Wasm. The initial intermediate artifact
from `0886cad` was superseded by the strict admission-order fix and must not be
used. Final two-build reproducibility and export gates below remain mandatory
from the final clean revision; artifact identity alone is insufficient.

## Production Candid

The exact backend interface is:

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

No backend application update method was introduced. `preview_subscription`
uses the same parser/resolver as admission, performs no awaits or writes, and
reports syntax/resolution, never funding or admission. Production exports are
limited to these three queries; the frontend still exposes exactly
`http_request` as its application method. The binary audit also rejects debug
markers and a compiled-in canonical Event Horizon instance identity.

The canonical subscriber interface is:

```candid
type PokeTarget = variant { subaccount : nat64; neuron_nonce : nat64 };
type PokeMatch = record { target : PokeTarget; first_block_index : nat64; max_amount : nat };
// Event Horizon always sends this payload. A wake-up-only subscriber may
// instead implement `poke : () -> ()`; see docs/subscriber-guide.md.
type SubjectPokeMatch = record { subject : principal; target : PokeTarget; first_block_index : nat64; max_amount : nat };
service : { poke : (vec PokeMatch, opt vec SubjectPokeMatch) -> (); poke_subjects : (vec SubjectPokeMatch) -> (); }
```

The first vector contains only self-subject matches. Self-only and global-only
calls have a null second argument. Mixed self/external or genuine global/external
activity uses one normal two-argument call. External-only activity uses only
`poke_subjects`; missing/trapping methods do not cause a fallback or stop other
attempts. Old one-vector, old two-field-record, and zero-argument `poke` receivers
retain their intended self/global compatibility. External-aware receivers need
both methods. There is no capability negotiation or sender retry.

The actual global-match flag controls global-first dispatch, with deterministic
principal order within each phase. Payloads retain deterministic target order
within each vector, and external vectors additionally sort by subject principal.
At most one call is attempted per subscriber per completed poll. The combined
self/external first-retained hint cap remains 256. Remote delivery/execution
order, Index visibility and Index completion remain unguaranteed.

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

## Regression and upgrade evidence

The managed suites enumerate 137 Rust tests, 138 frontend tests and 119 PocketIC
cases. Complete gate results are reported alongside the final clean revision;
this coverage record does not substitute for executing those commands.

New dedicated scenarios cover:

- Alias/principal and explicit-self resolution, all declaration classes,
  malformed/unknown references, u64 endpoints, 256/257 range bounds, decimals,
  NNS/SNS derivation, and shared preview/admission parsing.
- Daily/cache/reserve behavior, failed refresh isolation from polling, additive
  publication and persistence, and unknown-alias non-replay.
- Both Ledger readers, A/B/C independently watching B.7, reverse A/B watching,
  different thresholds and independently correct first indices/maxima, multiple
  subjects, self/external mixtures, exact route spelling, overlapping ranges,
  archive gaps, multi-page aggregation and staged-page failure/recovery.
- Actual Candid and receiving canisters for every receiver shape, full-width
  indices and large Nat amounts, external-only safety, genuine global/external
  classification, method failures, 256 combined hints, and caller identity.
- Debounced preview gating, stale input/instance/disposal responses, unknown
  aliases and direct-principal fallback, byte bounds, subject roles and E routing.

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

Existing reader, archive, duplicate-block, admission, neuron, reserve, pricing,
funding and scheduler regressions remain in the complete managed suite. No test
asserts remote cross-canister execution ordering.

## Resource measurements

The initial uncached 1,024-watcher/256-transfer workload exceeded PocketIC's
40-billion-instruction message limit. Reusing decoded account watchers within a
page removes repeated stable decoding; staged admissions invalidate the affected
cache immediately. Both Ledger readers now complete that workload without
resumable state, dropped subscriptions or altered pricing/limits.

Settled local cycle measurements (30 post-poll ticks; synthetic recipients,
1,024 independently stored watchers with raw thresholds 0–15 and transfer amount
20) were:

| Incoming transfers to one watched account | Shared ICP cycles | ICRC-3 cycles |
| --- | ---: | ---: |
| 1 | 5,904,612,876 | 5,911,610,415 |
| 32 | 6,132,808,879 | 6,142,259,611 |
| 256 | 7,780,162,512 | 7,814,511,298 |

These are local workload measurements, not mainnet forecasts or remote-delivery
proof. Exact instruction counts are not emitted by this fixture; completion is
checked under the ordinary message limit. Shared-account work remains
proportional to the number of interested relations. There are no subscriber
quotas or speculative new prices.

The existing global-fanout fixture measured 1,463,490,158 cycles for 256
recipients and 5,791,562,161 for 1,024. Compared with its previous recorded run
(1,431,240,087 and 5,662,738,686), this is approximately 2.3% higher. The normal
two-argument encoding adds 32 bytes to the representative self-only (98→130)
and global-only (44→76) payloads. A real combined 256-hint callback measured
8,511 bytes in the ICP, ICRC-3 and SNS receiving-canister tests. Subject principal
bytes additionally increase each external hint. Arbitrary-precision Nat means
payload sizes depend on actual amounts.

Stable allocation in these fixtures rises from 75,563,008 to 100,728,832 bytes
with the three new regions: 24 MiB additional initial bucket allocation. Seeding
1,024 external records fits within that allocation and caused no further stable
page growth. Records still consume space within those buckets. Call reserve
checks use the actual chosen method and newly encoded payload's `get_cost()`.

## Required clean-source gates and rollout

Run and retain results for:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run -p xtask -- test_unit
POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all
./tools/scripts/security-scan
./tools/scripts/source-manifest verify
git diff --check
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
```

The reproducibility command creates two independent clean BuildKit builders,
uses no build cache, and compares all resulting artifacts byte for byte. Final
artifact manifests, both production export audits and the frontend's embedded
backend pin must agree with the hashes above. Dependency security scans retain
the four existing documented advisory exceptions; no new exception was added.

Outstanding operator work requires separate approval: deploy/verify Jupiter's
alias API and E routing first; preserve existing state/cursors through an ordinary
backend upgrade; deploy the reviewed frontend with the matching backend pin;
verify ordinary ICP/cycles funding, cursors and notification behavior before and
after. External-aware recipients must implement `poke_subjects` before relying
on external-only acceleration. Do not assume existing Jupiter receivers already
do. No canister creation, mainnet deploy/upgrade/reinstall, controller change,
fund transfer or domain action was performed by this task.

Primary guidance consulted includes the current DFINITY stable-memory and
multi-canister skills, [ICP timers](https://docs.internetcomputer.org/concepts/timers/),
[Candid sequence compatibility](https://github.com/dfinity/candid/blob/master/spec/Candid.md),
[CDK call/cycle APIs](https://docs.rs/ic-cdk/latest/ic_cdk/call/struct.Call.html),
[ICP Ledger interface](https://github.com/dfinity/ic/blob/master/rs/ledger_suite/icp/ledger.did)
and [ICRC-3](https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-3).

## Files changed

```text
Cargo.lock
Cargo.toml
MANIFEST.sha256
README.md
SPEC.md
candid/jupiter-historian.subset.did
candid/subscriber.did
canisters/event-horizon/README.md
canisters/event-horizon/event_horizon.did
canisters/event-horizon/event_horizon_debug.did
canisters/event-horizon/src/aliases.rs
canisters/event-horizon/src/clients/historian.rs
canisters/event-horizon/src/clients/subscriber.rs
canisters/event-horizon/src/lib.rs
canisters/event-horizon/src/memo.rs
canisters/event-horizon/src/polling.rs
canisters/event-horizon/src/state.rs
canisters/event-horizon/src/subscription.rs
canisters/frontend/README.md
canisters/frontend/public/app.bundle.js
canisters/frontend/public/app.js
canisters/frontend/public/index.html
canisters/frontend/public/instances.js
canisters/frontend/public/memo.js
canisters/frontend/public/pricing-client.js
canisters/frontend/public/subscription-preview.js
canisters/frontend/test/generic.test.mjs
canisters/frontend/test/helpers.mjs
canisters/frontend/test/ledger-counter-integration.test.mjs
canisters/frontend/test/subject-memo.test.mjs
docs/architecture/economics-pricing.md
docs/architecture/production-surface.md
docs/architecture/stable-memory.md
docs/architecture/trust-model.md
docs/codex/permanent-aliases-subjects-release.md
docs/development/testing.md
docs/operations/deployment.md
docs/operations/operational-backend.md
docs/operations/reproducible-builds.md
docs/subscriber-guide.md
tests/fixtures/event_horizon_44d83c0_debug.provenance.txt
tests/fixtures/event_horizon_44d83c0_debug.wasm
tests/mocks/mock-historian/Cargo.toml
tests/mocks/mock-historian/src/lib.rs
tests/mocks/mock-subscriber-legacy/Cargo.toml
tests/mocks/mock-subscriber-legacy/src/lib.rs
tests/mocks/mock-subscriber-subjects/Cargo.toml
tests/mocks/mock-subscriber-subjects/src/lib.rs
tests/pocketic/src/lib.rs
tools/audit-wasm.py
tools/static-check.py
```
