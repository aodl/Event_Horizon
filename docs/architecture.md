# Architecture

Event Horizon consists of an autonomous backend and a separately controlled certified informational frontend.

## Backend data flow

```text
ICP Ledger
   |
   | query_blocks (live history only)
   v
fixed-boundary poll
   |
   +-- Faucet-origin transfer to Event Horizon + valid memo
   |       |
   |       +--> Jupiter Historian exact-route admission check
   |                |
   |                +--> permanent watched-account subscription
   |
   +-- relevant transfer to admitted watched account
           |
           +--> transient BTreeMap<subscriber, BTreeSet<subaccount>>

poll reaches pinned boundary
   |
   +--> at most one bounded-response one-way poke(vec nat8) per subscriber
        containing sorted unique matched subaccounts
```

A watched account is relevant for every incoming transfer when its declaration omitted a threshold. When a threshold is present, relevance means `amount >= threshold`.

The subscriber's Ledger/Index reconciliation is always the correctness path.

## Autonomous scheduling

Production installs one one-shot event-poll timer and one one-shot funding-maintenance timer. Timers are reinstalled after upgrades. Polls cannot overlap; funding runs through a separate single-flight lease.

After each completed poll, the canister chooses the next shared cadence from its liquid cycles balance using the fixed hysteresis table in `SPEC.md`.

## Funding

Event Horizon's own default ICP balance is periodically swept to the CMC. The CMC payment transfer uses the established canister-top-up memo and principal-derived CMC subaccount. A single stable state slot preserves the transfer identity or accepted Ledger block across asynchronous failures.

## Failure boundaries

- **Poke failure:** discard; subscriber reconciliation recovers completeness.
- **Historian unavailable/incomplete:** do not admit from that payout; later perpetual payout is a natural later opportunity.
- **Ledger archive gap:** log, skip to live history, continue.
- **CMC transport/Processing:** retain the same notify/transfer identity and try again on the next funding maintenance.
- **CMC explicit refund:** clear; refunded ICP is naturally swept later.
- **CMC terminal protocol failure:** log and clear; there is no administrative recovery endpoint.
