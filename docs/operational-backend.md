# Operational backend

## Poll lifecycle

A first successful zero-length Ledger read bootstraps prospectively. Each later poll pins the first response's exclusive `chain_length`, processes only that interval, and persists page progress. Proven archived prefixes are logged and skipped without archive calls.

Single-account and expanded-range matches accumulate actual sorted subaccounts per subscriber. If at least one transaction was processed, the backend adds admitted global subscribers once after reaching the boundary. It sends at most one poke per subscriber: a non-empty account set wins; otherwise a global match sends an empty vector.

## Admission

Only a payout from the configured Faucet default account to Event Horizon can propose admission. Historian must verify the exact memo route, a complete view, and a total at least equal to the current account/range/global requirement. Pricing must have initialized. A valid range expands inclusively into at most 256 watched-account merges. Failed evaluation creates no retry queue; a later Faucet payout is another opportunity.

## Pricing lane

An independent one-shot timer handles daily CMC observations and exact freeze/effective timestamps. It prunes observations older than the 1,461 UTC-day window before epoch work, records no more than one success per UTC date, and persists one attempt per date to avoid same-day retry loops. Upgrade startup replaces timer slots and resumes this lane once.

## Funding lane

Hourly funding remains separate and is the only money-moving worker. Each opportunity records the current liquid balance into the surplus policy's seven-day observed minimum, then resumes the single durable FundingStateV2 before considering any new raw balance. A successful mint immediately recalculates polling cadence from liquid cycles. Pricing queries do not alter funding state.

With no compiled surplus destination, the policy resets to level zero and the legacy single CMC transfer is unchanged. When enabled and currently at least 150 T, a new balance may be split after reserving two current Ledger fees. The exact retained amount, surplus amount, fees, and deterministic identity are persisted before the retained legacy transfer. CMC `Processing` keeps the notification and plan pending. Refund or terminal error cancels the planned surplus.

Only successful CMC minting permits the next step. The lane re-reads liquid cycles; below 150 T it cancels the surplus leg. Otherwise it persists and sends the surplus transfer with memo `SURPLUS1`. Acceptance/duplicate clears the state, ambiguous outcomes retry the same identity, clean/no-debit outcomes replan later, and an expired identity logs once and clears. New ICP arriving during any pending state waits for a later plan.
