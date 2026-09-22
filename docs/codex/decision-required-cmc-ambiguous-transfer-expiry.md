# Resolved decision: expired ICP transfer identity

## Decision

Event Horizon uses `Call::unbounded_wait` for the trusted ICP Ledger legacy `transfer` call that moves ICP to the CMC account. The deterministic `created_at_time`, amount, fee, destination, memo, and source identity remain persisted in `TransferPending` until the Ledger returns a usable result. Ledger reads, Historian calls, subscriber pokes, and CMC notification remain bounded.

A successful transfer or `TxDuplicate` pins the accepted block index in `NotifyPending` for `notify_top_up`. A non-clean transport reject or response decode failure retains the same transfer identity. A clean reject establishes that this call did not execute and permits replanning from live balance and fee data.

If retrying an existing identity eventually returns `TxTooOld`, Event Horizon logs one concise `CMC_TRANSFER_IDENTITY_EXPIRED` event and clears the plan. A later maintenance run re-reads the live balance and fee and creates a fresh deterministic identity. This choice preserves autonomous funding liveness without an Index, archive traversal, administrative recovery endpoint, journal, or transaction search.

## Why this resolves the controllerless concern

The earlier implementation used bounded wait for the value-moving Ledger call. A `SYS_UNKNOWN` response could hide an ordinary accepted transfer and leave only duplicate-window recovery. Guaranteed-response semantics remove that ordinary timeout ambiguity for the trusted ICP Ledger: once delivered and completed without a trap, its response is delivered to Event Horizon.

The persisted identity and duplicate handling remain defense in depth for retries after clean failures, rejects, upgrades, and other interruptions. The subsequent CMC notification continues from the accepted block index and stays bounded because the transfer itself is already identified.

## Narrow residual risk

After an extraordinary non-clean Ledger failure, a previous payment could theoretically have been accepted without its block being recoverable. If the persisted identity later expires, clearing it can strand that one CMC payment. Completely eliminating this pathological ambiguity would require durable transaction lookup, archive traversal, or operator recovery machinery deliberately excluded from Event Horizon v1.

This residual risk is accepted in favor of autonomous liveness. The exceptional expiry log preserves operational evidence without adding a production control surface.

## History

The initial validation identified that bounded-wait ambiguity could outlive the Ledger deduplication window. The prior decision record offered three choices: accept a narrow residual risk, add recovery machinery, or preserve ambiguity indefinitely and lose funding liveness. This hardening pass adopts the first choice together with unbounded wait for the narrowly trusted value-moving call.
