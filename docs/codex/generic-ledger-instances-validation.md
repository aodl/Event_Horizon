# Generic ledger instances corrective validation

## Revision information

- Original baseline: `c0afddded408bb1c9963b235d91b32478963bb79`
- Corrective starting HEAD: `bd0a8898fcee611625a12579a07c74e0783a1899`
- Branch: `codex/generic-ledger-instances`
- Corrective implementation revision: pending final commits

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
page commits after complete decoding; an unexplained hole or malformed required
transfer preserves its live cursor. Reserve Protection is local suppression,
does not mutate cursors, and does not emit a remote-outage transition.

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
- PocketIC integration: 51 passed in the final complete current-tree run.
- Security gate: passed. `cargo audit` reported the four documented allowed
  maintenance warnings; cargo-deny advisories/bans/licenses/sources passed;
  npm audit reported zero vulnerabilities; OSV reported no unfiltered issues.
- Production backend export audit: passed in
  `production_wasm_exposes_only_instance_and_pricing_queries` and static checks.
- Production frontend export audit: passed in static checks.
- `DFX_IDENTITY=codex_local icp build -e local`: passed.

Canonical Docker reproducibility is not confirmed. The first full validation
attempt completed tests and security but Docker failed during the pinned Debian
snapshot `apt-get` step with exit 100. A retry failed identically. Plain build
output showed all Debian `InRelease` signatures rejected as invalid inside
Docker; the host filesystem was 98% full. No global Docker/system cleanup was
performed and apt signature verification was not weakened.

Consequently the local files under `release-artifacts/` are not accepted as
final canonical outputs, the static frontend registry intentionally retains a
null expected backend hash, and no backend hash, frontend hash, source-manifest
finalization, or release archive hash is claimed in this report.

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
- [ ] Final registry contains new canonical backend hash — blocked by canonical
  Docker build failure; remains null by policy.
- [ ] Two canonical builds are byte-identical — blocked by Debian signature
  verification failure before the build could complete twice.

## Safety

No deployment, reinstall, upgrade, transfer, top-up, controller change,
blackholing, alias publication, or other mainnet mutation was performed.
