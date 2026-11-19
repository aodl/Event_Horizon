# Ledger archive recovery

The starting revision was `2a9d2235f0e7a45aaa112eeff5287d7b6d836e6e`. The final backend reader, specification amendment and initial regressions are in `a4c3fe3c3e699804adf03aa5d564667441362c2f`. This companion revision adds validation and fixture checks.

Sections 6–8 and 12 of the specification now require actual unread archive
blocks to be fetched and processed through the existing admission and
matching rules. Upgrades resume from retained cursors. Previously skipped
blocks below those cursors remain outside recovery. Fresh installations start
at their configured root tips.

The same PocketIC assertions ran against the unchanged debug reader and the
updated reader. Fixtures supplied real typed callbacks at separate canisters
and omitted the archived blocks from the root-local responses.

| Scenario | Before | After |
| --- | --- | --- |
| Shared ICP payout and surrounding activity | Archived Faucet payout failed to admit | Payout admitted. Only later activity matched, with first index and maximum from different blocks |
| Separate ICP admission | Archived Faucet payout failed to admit | Payout admitted while the observed cursor remained independent |
| Generic ICRC-3 observation | Receiver reported live index 257 | The authenticated notification reported archived index 256 and the live maximum of 18,446,744,073,709,551,616 |
| Empty archive and a live suffix | Both shared cursors advanced from 1 to 3 | Both held at 1. A later poll processed the recovered suffix |

Two protocol-specific retrieval helpers select the source covering the next
unread index. They replace descriptor coalescing and skip-only logging.
Root-local pages need no callback. Each archive prefix adds the calls needed to
resolve that prefix. PocketIC observed 16 calls for an unresolved self-referral,
then progress on an ordinary poll once the route was corrected. Root and
archive reads share each client's bounded wait and actual call-cost guard.
Page and descriptor counts remain bounded at 256.

The stable schema and public Candid are unchanged. Archive routes are transient
and originate in ledger replies. The reader retains the existing timers and
Index-independent architecture. No replay or administrative endpoint or
notification queue was added.

Availability and the liquid-cycle reserve can hold a cursor in place. ICRC-3
routes deeper than 16 calls remain stalled until the route or implementation
changes. Cursors retain their unsigned 64-bit domain and amounts remain
arbitrary-precision. Notifications remain best effort. A failed scan can lose
transient hints for already committed prefixes, and unavailable Historian
evidence can still prevent admission under the existing rules.
