# Decision required: ambiguous ICP transfer after the Ledger deduplication window

## Observed problem

`TransferPending` preserves the exact legacy transfer identity after a lost Ledger response. A prompt retry safely recovers the original block through `TxDuplicate`. If retries remain unavailable beyond the Ledger's 24-hour acceptance window, the same request returns `TxTooOld`. The current code treats that as a definite no-debit result and clears the plan.

## Evidence / failing test

The [ICP Ledger Candid](https://github.com/dfinity/ic/blob/master/rs/ledger_suite/icp/ledger.did) describes `TxTooOld` as an expired request and `TxDuplicate` as the original accepted block index. The existing PocketIC test proves prompt duplicate recovery; it does not prove recovery after expiry. The [ICP idempotency guidance](https://legacy.internetcomputer.org/docs/building-apps/best-practices/idempotency) notes that an accepted transfer with a lost response may become unresolvable after the deduplication window. The [CMC Candid](https://github.com/dfinity/ic/blob/master/rs/nns/cmc/cmc.did) requires the exact block index for `notify_top_up`.

## Current specified behavior

`SPEC.md` requires autonomous legacy transfer to CMC, duplicate-safe recovery, and no archive traversal, Index dependency, operator recovery endpoint, or financial journal.

## Why a code-only fix is unsafe

After expiry, `TxTooOld` proves only that the retry did not debit. It cannot prove whether the original call debited before its response was lost. Clearing the plan may strand an accepted CMC payment without a block index for notification. Keeping the plan indefinitely protects against a mistaken new spend but can stop future funding sweeps. Neither choice satisfies autonomous recovery in every case.

## Smallest viable alternatives

1. Accept and document this rare bounded-window loss of autonomous recovery, with operational monitoring before controller removal.
2. Amend the protocol to permit a narrowly scoped, durable payment lookup or recovery mechanism capable of finding the accepted block after expiry, including archived history when necessary.
3. Amend the protocol to preserve the ambiguous state indefinitely and explicitly accept funding liveness loss until an external resolution.

## Recommendation

Decide the required behavior for a transfer whose response remains ambiguous after 24 hours before calling the funding lane ready for controllerless operation. The current code remains unchanged pending that decision.
