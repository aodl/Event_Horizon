# Generic ledger instances corrective validation

## Target-aware u64 and nervous-system extension

The final schema uses canonical decimal `u64` numeric targets encoded as `24 zero bytes || N.to_be_bytes()`. Inclusive ranges may begin anywhere and expand to at most 256 watched accounts. `n<N>` uses the shared SHA-256 `neuron-stake` controller/nonce derivation. ICP resolves those accounts under fixed NNS Governance; an SNS-aware generic instance stores immutable `sns_root`, verifies Root's Ledger/Governance tuple and the Ledger's Governance-default minting account, then caches Governance in the observed profile. Non-SNS generic instances retain `neuron_governance = null`.

The callback reports deterministic `{ target; max_amount : Nat }` matches. Maxima are per individual qualifying transfer in raw observed-token atomic units; specific vectors take precedence over global `poke([])` and retain at most 256 targets per subscriber/poll. Generic `Nat` never narrows through `u64`. The same canonical backend Wasm is configured for ICP/NNS, generic non-SNS, and verified generic SNS modes; SNS-WASM is review evidence only, never a runtime dependency.

## Revision information

- Original baseline: `c0afddded408bb1c9963b235d91b32478963bb79`
- Corrective starting HEAD: `bd0a8898fcee611625a12579a07c74e0783a1899`
- Branch: `codex/generic-ledger-instances`
- Canonical artifact source revision:
  `efac79c04070664fecfff456d059e8610281c3c7`

No migration from the acceptance deployment or an intermediate generic schema
was added. A deliberate reinstall remains the accepted transition.

## Canonical ICP protocol

Operator-supplied read-only mainnet evidence from 2026-09-26 establishes that
canonical ICP Ledger `ryjl3-tyaaa-aaaaa-aaaba-cai` advertises exactly ICRC-1,
ICRC-2, and ICRC-21. A query-mode `icrc3_supported_block_types` call was
rejected with `IC0536`: `Canister has no query method
'icrc3_supported_block_types'`. Codex did not generate that live evidence; it
was supplied from the operator's network-enabled environment. Pinned official
DFINITY source/Candid evidence is retained in
`decision-required-canonical-icp-ledger-contract.md`.

Canonical ICP uses the fixed legacy `query_blocks` adapter. One physical page
read supplies Faucet admission and ICP-instance observation in ledger order.
`icp_instance_uses_one_page_read_for_both_roles` instruments legacy page calls.
`shared_icp_scan_applies_admission_in_legacy_block_order` proves that admission
starts matching only later blocks in the same authoritative stream.
Global ordering is proved separately: `shared_global_admission_block_does_not_self_wake`,
`shared_global_admission_matches_later_same_poll_activity`, and
`shared_global_activity_before_admission_is_not_retroactive` cover the admission
boundary, while `existing_shared_global_many_blocks_coalesce_to_one_poke`
proves existing-global activity and one-poke coalescing. Static checks also
prove `legacy_page` does not enumerate the global registry per live block.

## Generic non-ICP protocol

Every non-ICP Observed Ledger must advertise ICRC-1 and ICRC-3 and expose
`1xfer`; `2xfer` is optional. The profile caches bounded symbol, decimals, and
transfer-from capability. The narrow decoder recognizes `1xfer`, advertised
`2xfer`, and the reviewed absent-`btype`/`tx.op=xfer` legacy form. Identified
malformed transfers fail closed; unknown valid types are global activity only.

`generic_profile_requires_icrc1_icrc3_and_1xfer_but_not_2xfer` proves each
mandatory capability and optional `2xfer`. `transfer_from_global_other_and_
malformed_retry_semantics` proves transfer-from, unknown-block, global, and
malformed retry behavior.

## Same-Wasm proof

One backend Wasm contains both adapters. Equality with the compiled canonical
ICP principal selects the shared legacy reader; every other principal selects
ICRC-3 observation plus legacy ICP admission. There is no fallback, mutable
reader choice, or arbitrary legacy-ledger support.

`same_wasm_supports_independent_observed_ledger_and_fixed_icp_funding` installs
the same debug backend bytes with two different observed-ledger principals,
checks their SHA-256 equality, proves independent observed activity, and proves
that funding still calls only the Protocol ICP Ledger. The production artifact
remains a single `event_horizon.wasm`.

## Stream and stable-state semantics

Fresh cursors are prospective: legacy streams use `chain_length`; generic
streams use `log_length`. Both shared and distinct modes establish the Protocol
ICP starting cursor before profile discovery. In distinct mode, observed
scanning uses subscriptions present at poll start, then admission runs, then the
already-determined match set is delivered.

Only explicit contiguous archive ranges may advance a cursor and archive
callbacks are never called. Archive-only progress is not live activity. A live
page publishes its admission mutations and corresponding cursor together only
after all required page processing. Historian-verified admissions remain in a
page-local staging overlay and are not durable ahead of that cursor. Later
shared-page blocks can match the staged account overlay, while the admission
block and earlier activity remain non-matching. An unexplained hole or malformed
required transfer preserves its live cursor. Reserve Protection is local
suppression, does not mutate cursors, and does not emit a remote-outage
transition.

Shared cursor flags and values must agree. Divergence logs one exceptional
diagnostic and fails closed without a fetch, rewind, mutation, or poke.

The final ID 1 watched-account keys are:

```text
0x00 || 32-byte ICP AccountIdentifier
0x01 || principal-length byte || principal || effective 32-byte ICRC subaccount
```

This collision-separates the irreversible legacy ICP representation from ICRC
Account semantics while retaining one direct stable-map lookup. There is no
abandoned-key migration. Threshold values remain arbitrary-precision `Nat`;
ICP uses eight decimals and generic precision comes from `icrc1_decimals`.

Evidence includes:

- `fresh_profile_failure_preserves_prospective_icp_admission_start`
- `distinct_ledger_admission_is_not_retroactive_within_a_poll`
- `distinct_stream_failures_do_not_suppress_the_other_stream`
- `archive_only_progress_does_not_poke_a_global_subscriber`
- `unexplained_ledger_hole_preserves_cursor_until_archive_evidence_arrives`
- `live_page_cursor_is_atomic_across_a_late_malformed_block`
- `staged_account_admission_is_not_durable_before_shared_cursor_commit`
- `staged_global_admission_is_not_durable_before_shared_cursor_commit`
- `staged_distinct_admission_is_not_durable_before_admission_cursor_commit`
- `reserve_protection_does_not_mutate_cursors_or_log_remote_outages`
- `shared_cursor_divergence_fails_closed_without_rewind_or_fetch`
- `subscription_and_cursor_survive_upgrade`

## Frontend

The registry is static: ICP is live/canonical with alias `X`; IO is
planned/canonical with intended alias `I` and unset principals/hash. A live
entry must match `get_instance.observed_ledger` and have an observed profile
before memo construction is enabled.

The UI uses the official `@icp-sdk/core` Principal parser, trims UI whitespace,
rejects malformed checksum/encoding plus anonymous and management principals,
emits the compact representation already accepted by the backend, and applies
the 32-byte limit to the final memo. Runtime ledger/error text uses
`textContent` through `renderRuntimeError`.

Frontend cases `official Principal parsing matches backend subscriber rules`,
`memo limit applies after validated Principal normalization`, and `runtime
error rendering uses a text sink` cover validation parity, exact/over-limit
memos, and malicious-looking runtime text.

## Validation results

Completed on 2026-09-26:

- Static/source checks: passed.
- Rust formatting and Clippy: passed.
- Backend unit tests: 53 passed.
- Frontend tests: 9 passed.
- PocketIC integration: 58 passed in the final complete current-tree run.
- Security gate: passed. `cargo audit` reported the four documented allowed
  maintenance warnings; cargo-deny advisories/bans/licenses/sources passed;
  npm audit reported zero vulnerabilities; OSV reported no unfiltered issues.
- Production backend export audit: passed in
  `production_wasm_exposes_only_instance_and_pricing_queries` and static checks.
- Production frontend export audit: passed in static checks.
- `DFX_IDENTITY=codex_local icp build -e local`: passed.
- Full validation stages passed. The aggregate `validate` run completed tests and
  security, then its first no-cache build hit an invalid-signature error caused
  by exhausted Docker build cache space. Only unused generated build cache was
  pruned; the unchanged `repro` retry and final `canonical` stage passed.
- Two independent `docker build --no-cache` artifact sets: byte-identical.
- Canonical backend SHA-256:
  `0f50ef898f5fc196fabfcaf4d7f554b8a3b56bdae175b524b53e4d3d00d5dead`.
- Canonical frontend SHA-256:
  `3843156571a9ca2c409f6a92c87e0672e3d2a7d89ed158fb5068b49725f02a74`.
- Deterministic canonical-artifact archive SHA-256:
  `dd2a3a9953d0d2a3f106b5e53bfd0e3a5412fa4c093bbded3796ff0acb9a55e0`.
- Canonical artifact manifest: all three entries verified.
- Source manifest: regenerated after final evidence and verified before the
  final evidence commit.

The static ICP registry pins the exact canonical backend hash above. The
backend export audit passed with ten total Wasm exports and exactly
`canister_query get_instance` and `canister_query get_pricing` as application
methods. The frontend audit passed with seven total exports and exactly
`canister_query http_request` as its application method.

## Required evidence checklist

- [x] Live canonical contract recorded — resolved-contract report.
- [x] ICP mock has no successful ICRC-3 endpoint —
  `canonical_icp_mock_does_not_expose_icrc3`.
- [x] Generic mock is a separate package/artifact — `mock-icrc3-ledger`.
- [x] Same Wasm runs ICP and non-ICP modes — `same_wasm_supports_...`.
- [x] ICP uses one legacy page fetch — `icp_instance_uses_one_page_read_...`.
- [x] Non-ICP uses ICRC-3 observation plus legacy admission — same-Wasm and
  distinct-order tests.
- [x] Observed asset never funds Event Horizon — same-Wasm funding assertion.
- [x] Profile outage cannot skip later payout — `fresh_profile_failure_...`.
- [x] Distinct failures are independent — `distinct_stream_failures_...`.
- [x] Shared order controls matching — `shared_icp_scan_applies_...`.
- [x] Distinct admission is non-retroactive — `distinct_ledger_admission_...`.
- [x] Archive-only progress never wakes global — `archive_only_progress_...`.
- [x] Unexplained gap preserves cursor — `unexplained_ledger_hole_...`.
- [x] Malformed transfer preserves cursor — `live_page_cursor_is_atomic_...`.
- [x] Verified admission never becomes durable ahead of its legacy cursor —
  `staged_account_admission_is_not_durable_before_shared_cursor_commit`,
  `staged_global_admission_is_not_durable_before_shared_cursor_commit`, and
  `staged_distinct_admission_is_not_durable_before_admission_cursor_commit`.
- [x] Reserve Protection logs no false outage — `reserve_protection_...`.
- [x] Shared divergence fails closed/no rewind — `shared_cursor_divergence_...`.
- [x] Final-schema upgrade causes no duplicate poke —
  `subscription_and_cursor_survive_upgrade`.
- [x] ICP default/numbered/range watched accounts — admission, multi-account,
  maximum-range, and range-overlap tests.
- [x] Generic ICRC Account and zero normalization — transfer test plus
  `absent_and_zero_icrc_subaccounts_have_the_same_key`.
- [x] Arbitrary-precision thresholds and precision sources — backend decimal
  tests and generic readiness test; ICP integration uses eight decimals.
- [x] `1xfer`, optional `2xfer`, and unknown activity — generic readiness and
  transfer/global/malformed test.
- [x] ICP mint/burn/approve global-only behavior —
  `icp_mint_burn_and_approve_are_global_only_activity`.
- [x] Specific/global precedence and maximum one poke — global precedence and
  coalescing tests.
- [x] Real frontend Principal parsing/parity/special-principal rejection —
  frontend Principal and memo-limit cases plus backend memo unit tests.
- [x] Unsafe runtime text stays text — runtime text-sink frontend case.
- [x] `SURPLUS_CANISTER=None`, exact production surfaces, and unset IO IDs —
  static checks and production export regression.
- [x] Final registry contains new canonical backend hash — frontend registry
  and canonical frontend artifact.
- [x] Two canonical builds are byte-identical — `xtask validate` reproducibility
  stage compared every emitted artifact from two `--no-cache` Docker builds.

## Safety

No deployment, reinstall, upgrade, transfer, top-up, controller change,
blackholing, alias publication, or other mainnet mutation was performed.
