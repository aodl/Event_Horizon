# Reader contract hardening validation

## Scope and starting evidence

- Starting revision: `dca1caa8ad62fa26df316f59857c9b180143addd`, with a clean worktree. The newer committed artwork and scrollbar revisions were preserved.
- Reviewed ancestor: `d06bbfc5661752c9d414ccc0a38d09e83c87af74`.
- Supplied deployed backend hash, used only as comparison context: `7c2aac3be594893c0bddaa3f0c6738ea2161a2d23b87beec7268068d364e3b51`.
- Official legacy contract reference: DFINITY `ic` revision [`6a36bed9dad2542f7b9f508560301703b1936ede`](https://github.com/dfinity/ic/blob/6a36bed9dad2542f7b9f508560301703b1936ede/rs/ledger_suite/icp/ledger/ledger_candid_backwards_compatible.did). Its `QueryBlocksResponse` says that `first_block_index` is unspecified when `blocks` is empty and that archive entries cover subranges of the requested range.
- Actual prepatch debug Wasm: `tests/fixtures/event_horizon_reader_baseline_dca1caa_debug.wasm`, SHA-256 `54578ead9519a5b63372e5d76997adafe1b2da46f2d3fec7568fafd3baeb9092`.

The confirmed defect was the unconditional legacy `first_block_index` comparison on an empty live vector. The supplied production logs strongly suggest the resulting page-at-a-time failure pattern, but the exact mainnet reply was not captured and is not represented here as a captured fixture.

No stable-memory ID, stable type, constructor, public Candid method, cadence, pricing rule, funding behavior, archive-call policy, or delivery guarantee changed. The supplied artwork material was not integrated or modified.

## Production corrections

- Legacy empty pages now ignore `first_block_index`; nonempty replies retain the existing live-hole check.
- Legacy and ICRC-3 archive ranges use checked interval arithmetic and are clipped to the actual outstanding request and pinned boundary. Range-count and excessive-live-vector checks occur before structural data is used.
- Both readers continue page-by-page to the original pinned boundary. Only explicit contiguous evidence commits archive progress; no-progress replies fail.
- ICRC-3 live blocks remain sortable out of order. Identical relevant duplicate IDs collapse to one; conflicting relevant duplicates reject the live page before publication.
- A present non-text ICRC-3 `btype` is malformed. Only an absent discriminator enables the legacy `tx.op = "xfer"` fallback.
- A live frontend selection immediately renders the existing disabled placeholder before awaiting the backend, while the generation guard continues to reject stale completions.

## Before/after regression

The mock first returned the production-like first-live index without clamping it to the requested end. Against the unchanged reader, `shared_icp_multi_page_archive_catch_up_ignores_empty_first_index` failed after exactly one 256-block archive page: the shared cursor was `257`, not the pinned boundary `770`. After changing only the empty-page interpretation, the identical test passed and used four legacy requests in one poll to traverse three archive-only pages and process the live tail.

The tracked baseline Wasm reproduced the old failure in PocketIC with a nonzero shared cursor. An argument-free upgrade to the fixed debug Wasm retained immutable configuration, pricing, account/global subscriptions, and both cursors, then completed the remaining archive backlog without bootstrap or reset.

## Coverage map

| Contract edge | Native evidence | Reader mapping |
| --- | --- | --- |
| Empty chain, prospective bootstrap, caught-up scan | `same_wasm_supports_icp_generic_and_verified_sns_modes`; `fresh_profile_failure_preserves_prospective_icp_admission_start`; ordinary repeated polls | Shared legacy and distinct streams |
| Complete/partial archive evidence | `archive_interval_coverage_is_bounded_contiguous_and_order_independent`; multi-page PocketIC cases | Common interval helper plus all three readers |
| Empty first index: zero, request end, first local, `u64::MAX` | `shared_icp_empty_pages_ignore_all_valid_first_index_variations` | Shared legacy; the same private legacy worker serves separate admission |
| Empty response with no evidence; genuine live hole | `distinct_stream_failures_do_not_suppress_the_other_stream`; `unexplained_ledger_hole_preserves_cursor_until_archive_evidence_arrives` | Separate and shared legacy |
| Out-of-order, adjacent, overlapping, repeated, zero-length, and gapped ranges | table-driven interval unit test | Shared arithmetic used by legacy and ICRC wrappers |
| Request clipping, boundary at/below cursor, near-`u64` values, overflow | interval table plus legacy/ICRC wrapper rejection tests | Both reader families |
| Mixed archived prefix/live suffix and short final pages | shared, separate, and generic multi-page live-tail tests | All three readers |
| Structural invalidity before mutation | `invalid_icrc_envelope_cannot_commit_archive_evidence_before_rejection` | Generic ICRC-3 |
| Valid archive proof before malformed live value | `valid_icrc_archive_prefix_survives_malformed_live_suffix` | Generic ICRC-3 |
| Earlier page durable when later page fails | `admission_at_page_end_survives_next_page_failure_and_matches_on_retry` | Shared legacy publication boundary |
| Poll boundary remains pinned while Ledger grows | `transaction_after_pinned_boundary_waits_for_following_poll` | Shared legacy; boundary logic is common to both scans |
| Generic duplicate IDs | unit test plus `icrc_duplicate_ids_deduplicate_identical_values_and_reject_conflicts` | Generic ICRC-3 |
| Multi-page archive catch-up | shared legacy, separate admission, and generic ICRC-3 named tests | All three reader roles |
| One actual Economy scheduled execution | `economy_scheduler_traverses_all_shared_archive_pages_in_one_execution` | Real polling wrapper, unchanged cadence/rearm |
| Independent stream failures | strengthened `distinct_stream_failures_do_not_suppress_the_other_stream` | Malformed observed page and targeted legacy query rejection, with cursor/log/call evidence |
| Sparse failure/archive logging | `sparse_reader_failure_and_archive_logs_follow_episode_boundaries` | Failure suppression/recovery, independent HEALTH, one archive episode log |
| Admission and matching publication | staged-admission tests, page-end/next-page test, threshold/max/coalescing tests | Shared and separate admission |
| Historian boundaries | strengthened exact-evidence test and `historian_outage_does_not_halt_observation_or_create_admission_retry` | Wrong source/destination/route, incomplete/faulted data, no retry queue |
| Official legacy wire projection | `official_empty_legacy_reply_decodes_into_consumer_projection` | Official full reply encodes; narrower production type decodes |
| Decoder matrix | five table-driven `clients::icrc3` tests | Discriminator, tx/field/account shapes, normalization, arbitrary `Nat`, extra fields |
| Retained-state upgrades | expanded `subscription_and_cursor_survive_upgrade`; baseline-to-fixed test | Current-to-current and actual prepatch-to-fixed |
| Frontend pending selection | `switching live instances immediately clears stale memo and funding output` | Pending, failure, and late stale completion |

The arithmetic table is intentionally shared instead of multiplying every integer permutation across every PocketIC mode. PocketIC is reserved for cursor mutation, publication, inter-canister failure, scheduler, callback, and upgrade effects.

## Mock and fixture integrity

The legacy mock now reports the first available live index by default, even when it lies beyond an individual request window. A bounded override keyed by requested start changes only the reported field of an empty reply. Separate bounded controls force selected legacy query rejections and the two specific malformed ICRC envelopes used by structural/duplicate tests. They do not fabricate successful archive calls, mutate state during a query, or import the production validator.

The legacy wire fixture independently mirrors the official interface, including callback function references and the optional certificate. It encodes an empty reply with an unspecified `first_block_index` and decodes it into the production consumer's narrower Candid projection.

## Validation and artifact evidence

Targeted Rust unit, Node, and isolated PocketIC regressions passed during implementation. Final aggregate counts, source/artifact revisions, canonical hashes, frontend pin agreement, and release-gate results are recorded here after the clean artifact-source commit and canonical builds.

## Remaining limits

Event Horizon still does not call archive canisters or reconstruct archived payouts. Explicitly archived intervals can therefore contain missed activity, particularly after downtime or in Economy mode. Pokes remain one-way, best effort, and transient across a failed overall scan. HEALTH indicates maintenance activity rather than a successful complete scan, and failure-log suppression is not recovery evidence. Subscribers must retain independent authoritative reconciliation.
