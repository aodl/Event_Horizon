# Operational backend

## Poll lifecycle

A first successful zero-length Ledger read bootstraps prospectively. Each later poll pins the first response's exclusive `chain_length`, processes only that interval, and persists page progress. Proven archived prefixes are logged and skipped without archive calls.

Single-account and expanded-range matches accumulate actual sorted subaccounts per subscriber. If at least one transaction was processed, the backend adds admitted global subscribers once after reaching the boundary. It sends at most one poke per subscriber: a non-empty account set wins; otherwise a global match sends an empty vector.

## Admission

Only a payout from the configured Faucet default account to Event Horizon can propose admission. Historian must verify the exact memo route, a complete view, and a total at least equal to the current account/range/global requirement. Pricing must have initialized. A valid range expands inclusively into at most 256 watched-account merges. Failed evaluation creates no retry queue; a later Faucet payout is another opportunity.

## Pricing lane

An independent one-shot timer handles daily CMC observations and exact freeze/effective timestamps. It prunes observations older than the 1,461 UTC-day window before epoch work, records no more than one success per UTC date, and persists one attempt per date to avoid same-day retry loops. Upgrade startup replaces timer slots and resumes this lane once.

## Polling cadence

`T = 10^12 cycles`. The cadence input is the canister's **liquid cycles**: cycles immediately available to spend after outstanding-call reservations, not the larger total balance that may include reserved call balances.

| Mode | Added delay after completed poll | Enter at | Exit below |
| ------------------ | -------------------------------: | -------: | ---------: |
| Reserve Protection | ordinary polling suspended | — | 1 T |
| Economy | 1 hour | 2 T | 1 T |
| Standard | 10 minutes | 5 T | 3 T |
| Fast | 2 minutes | 10 T | 6 T |
| Very Fast | 10 seconds | 25 T | 15 T |
| Continuous | 0 | 100 T | 60 T |

Hysteresis prevents oscillation at entry thresholds. Event Horizon increases cadence immediately when a higher entry threshold is reached and may jump directly to the fastest enterable mode. When cycles fall, it remains in the current mode until that mode's lower exit threshold is crossed, then falls back as far as necessary. A canister at 5.27 T starting from a lower mode enters Standard mode. It remains Standard while its liquid balance is at least 3 T. Below 3 T it falls back to an appropriate lower mode.

These durations are **added delays after completed polls**, not exact poll intervals. Each poll performs its Ledger and inter-canister work before the next one-shot timer is scheduled. A single-flight guard prevents overlapping polls. Continuous means Event Horizon deliberately inserts no delay after a completed poll; polls still never overlap, and asynchronous IC execution naturally yields between calls.

Below 1 T liquid cycles, Reserve Protection suspends ordinary Ledger polling to protect protocol liveness. The canister does not stop: funding and other recovery-oriented maintenance remain available, and a short one-shot recheck timer continues evaluating the balance. A canister already in Reserve Protection does not resume Economy until it reaches the 2 T Economy entry threshold.

A successful CMC mint immediately reevaluates cadence from the newly available liquid cycles instead of waiting for the next completed poll. This polling table is independent of adaptive surplus policy: Continuous begins at 100 T, while 150 T is a surplus-health and transfer gate used only when the currently disabled surplus destination is enabled.

## Funding lane

Hourly funding remains separate and is the only money-moving worker. Each opportunity records the current liquid balance into the surplus policy's seven-day observed minimum, then resumes the single durable `FundingState` before considering any new raw balance. A successful mint immediately recalculates polling cadence from liquid cycles. Pricing queries do not alter funding state.

With no compiled surplus destination, the policy resets to level zero and the legacy single CMC transfer is unchanged. When enabled and currently at least 150 T, a new balance may be split after reserving two current Ledger fees. The exact retained amount, surplus amount, fees, destination account identifier, and fixed memo are persisted before the retained legacy transfer. CMC `Processing` keeps the notification and plan pending. A later runtime/compiled destination change applies only after the pending plan reaches Idle. Refund or terminal error cancels the planned surplus.

Only successful CMC minting permits the next step. The lane re-reads liquid cycles; below 150 T it cancels the surplus leg. Otherwise it adds a deterministic creation time and sends the surplus transfer using only the already-persisted account, memo `SURPLUS1`, amount, fee, and time. It never reconstructs a pending destination or memo from current configuration. Acceptance/duplicate clears the state, ambiguous outcomes retry the same identity, clean/no-debit outcomes replan later, and an expired identity logs once and clears. New ICP arriving during any pending state waits for a later plan.
