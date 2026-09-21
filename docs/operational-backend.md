# Operational backend

## Poll lifecycle

1. A fresh backend calls `query_blocks` with a zero-length range and records the returned `chain_length`. Historical blocks are not replayed.
2. Each later poll uses the first response's `chain_length` as a fixed exclusive boundary.
3. Pages are processed in ascending block order. The durable cursor advances after each successfully handled page.
4. Relevant destinations add their numbered subaccount to a transient `BTreeMap<subscriber, BTreeSet<subaccount>>`.
5. After the fixed boundary is reached, each subscriber receives at most one `poke(vec nat8)` attempt containing all distinct matched subaccounts for that poll.
6. The next mode is chosen from the canister's liquid cycles balance using the fixed hysteresis table.

Blocks arriving during a poll are deliberately left for the next poll.

A watched account without an explicit amount threshold matches every incoming transfer. A thresholded account matches inclusively at `amount >= threshold`, with `0.01 ICP` as the smallest explicit threshold.

## Admission

A transfer can propose a subscription only when its source is the configured Jupiter Faucet default ICP account and its destination is Event Horizon's default ICP account. Its ICRC-1 memo must parse as an Event Horizon subscription declaration.

Event Horizon then asks the configured Jupiter Historian for that exact `RawIcp` route. Admission requires a complete-from-genesis Historian view, no commitment-index fault, and at least 10 ICP cumulative qualifying endowment.

Historian read failure does not stop Ledger progress and does not create a retry queue.

## History gaps

Event Horizon never follows archive callbacks. If the required cursor is covered by an archived range, the backend logs the skipped interval, advances to the end of that archived range, and keeps crawling live history.

## Funding

Funding maintenance is independent from event polling. It periodically reads Event Horizon's default ICP balance and the current Ledger fee. When usable ICP exists it transfers `balance - fee` to the CMC account/subaccount for Event Horizon using the standard top-up memo, then calls `notify_top_up` with the resulting block index.

Only the in-flight financial identity is durable. There is no operator recovery API.
