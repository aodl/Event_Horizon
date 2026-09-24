# Architecture

Event Horizon consists of an autonomous backend and a separately controlled certified frontend.

## Poll lane

Each poll captures the first Ledger response's exclusive `chain_length` and never processes beyond it. Account transfers accumulate sorted subaccount hints per subscriber. The backend records whether any transaction was processed and, only after the boundary completes, incorporates the stable global-subscriber set once. Each subscriber receives at most one poke: a non-empty account hint wins over a global empty hint.

The reader uses no Index or archive traversal. A proven archived prefix is logged and skipped. Subscribers own authoritative reconciliation.

## Admission and storage

A Faucet-origin payout memo is parsed as either a global declaration or an account declaration. Historian must confirm the exact route and a complete cumulative total at least equal to the current corresponding price. Stable memory remains additive:

| ID | Contents |
|---:|---|
| 0 | existing metadata and Ledger cursor |
| 1 | existing account subscriptions |
| 2 | existing CMC conversion state |
| 3 | debug configuration in debug Wasm only |
| 4 | global subscriber set |
| 5 | daily pricing observations keyed by UTC day |
| 6 | current/frozen pricing state |

IDs 0–2 retain their validated encodings.

## Independent timer lanes

Production installs three one-shot timers: Ledger polling, hourly funding maintenance, and pricing maintenance. Each has a single-flight guard and replaces its prior timer when rescheduled. Pricing wakes at the next UTC day, freeze, or effective boundary. Funding and pricing use separate CMC calls and state.

## Frontend

Certified assets remain embedded in the Rust frontend Wasm. The frontend sets the ICP CLI deployment environment cookie on its document response; the bundled browser client reads `PUBLIC_CANISTER_ID:event_horizon` from that environment and directly invokes the backend's read-only `get_pricing` query. The frontend exports only the certified `http_request` query, so ordinary page views cannot trigger a frontend update or an inter-canister pricing call.

Daily pricing observation is a separate best-effort lane. It calculates the CMC query's current call cost before issuance and skips the day's attempt unless the liquid balance can retain the existing reserve floor after reserving that cost.
