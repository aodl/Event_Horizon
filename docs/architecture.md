# Architecture

```text
                         ┌─────────────────────┐
Jupiter Faucet ICP ─────►│ Admission / funding │
Canonical ICP Ledger ───►│                     │
Historian ───────────────►│   Event Horizon     │
CMC ─────────────────────►│                     │
                         │                     │
Observed ICRC Ledger ────►│ Trigger reader      │
                         └─────────┬───────────┘
                                   ▼
                              subscriber.poke
```

The observed asset can differ per immutable instance; the funding asset never does. Event Horizon has no token-specific business logic and requires no Index.

Event Horizon consists of an autonomous backend and a separately controlled certified frontend.

## Poll lane

Each poll captures the first Ledger response's exclusive `chain_length` and never processes beyond it. Account transfers accumulate sorted subaccount hints per subscriber. The backend records whether any transaction was processed and, only after the boundary completes, incorporates the stable global-subscriber set once. Each subscriber receives at most one poke: a non-empty account hint wins over a global empty hint.

The reader uses no Index or archive traversal. A proven archived prefix is logged and skipped. Subscribers own authoritative reconciliation.

## Admission and storage

A Faucet-origin payout memo is parsed as a global, single-account, or inclusive-range declaration. Historian must confirm the exact route and a complete cumulative total at least equal to the current corresponding price. An admitted range expands to at most 256 ordinary entries in the existing watched-account map; overlaps merge to the least restrictive permanent threshold. Ledger matching remains one destination lookup. Stable memory remains additive:

| ID | Contents |
|---:|---|
| 0 | existing metadata and Ledger cursor |
| 1 | existing account subscriptions |
| 2 | existing CMC conversion state |
| 3 | debug configuration in debug Wasm only |
| 4 | global subscriber set |
| 5 | daily pricing observations keyed by UTC day |
| 6 | current/frozen pricing state |
| 7 | authoritative funding state V2 for retained transfer, CMC notify, and surplus transfer |
| 8 | adaptive surplus epoch, observed minimum, and diversion level |

The first production schema uses IDs 0–1 and 3–8; ID 2 is intentionally unused. ID 7 contains the canonical `FundingState` and initializes directly to `Idle`. ID 8 defaults to disabled/uninitialized level zero because that initialization is part of the live surplus policy. Pricing continues storing account/global values internally; the exact range value is derived for admission and public reads.

## Independent timer lanes

Production installs three one-shot timers: Ledger polling, hourly funding maintenance, and pricing maintenance. Each has a single-flight guard and replaces its prior timer when rescheduled. Pricing wakes at the next UTC day, freeze, or effective boundary. Funding and pricing use separate CMC calls and state.

The hourly lane also takes one liquid-cycles observation for the surplus controller. It remains the only financial worker. A split plan is durably serialized as retained Ledger transfer, CMC notification, then—only after successful mint and a second 150 T check—surplus Ledger transfer. New ICP is never added to an in-flight plan.

## Frontend

Certified assets remain embedded in the Rust frontend Wasm. Its reviewed static registry contains ICP live and IO planned. A selected live backend supplies `get_instance` and ICP-denominated `get_pricing`; configuration must match before memo construction. No cookie, environment, URL, local storage, mutable service, or automatic discovery supplies principals. The frontend exports only certified `http_request`.

Daily pricing observation is a separate best-effort lane. It calculates the CMC query's current call cost before issuance and skips the day's attempt unless the liquid balance can retain the existing reserve floor after reserving that cost.
