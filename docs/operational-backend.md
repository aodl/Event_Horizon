# Operational backend

## Poll lifecycle

A first successful zero-length Ledger read bootstraps prospectively. Each later poll pins the first response's exclusive `chain_length`, processes only that interval, and persists page progress. Proven archived prefixes are logged and skipped without archive calls.

Account matches accumulate sorted subaccounts per subscriber. If at least one transaction was processed, the backend adds admitted global subscribers once after reaching the boundary. It sends at most one poke per subscriber: a non-empty account set wins; otherwise a global match sends an empty vector.

## Admission

Only a payout from the configured Faucet default account to Event Horizon can propose admission. Historian must verify the exact memo route, a complete view, and a total at least equal to the current account/global requirement. Pricing must have initialized. Failed evaluation creates no retry queue; a later Faucet payout is another opportunity.

## Pricing lane

An independent one-shot timer handles daily CMC observations and exact freeze/effective timestamps. It prunes observations older than the 1,461 UTC-day window before epoch work, records no more than one success per UTC date, and persists one attempt per date to avoid same-day retry loops. Upgrade startup replaces timer slots and resumes this lane once.

## Funding lane

Hourly funding remains separate. The legacy ICP Ledger value transfer uses unbounded wait and a persisted deterministic identity, followed by bounded CMC notification. A successful mint immediately recalculates polling cadence from liquid cycles. Pricing queries do not alter funding state.
