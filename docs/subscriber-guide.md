# Subscriber guide

Implement and authenticate this endpoint:

```candid
service : { poke : (vec nat8) -> (); }
```

## Declaration forms

```text
X.<subscriber>                         global Ledger trigger
X.<subscriber>.<subaccount>            all incoming transfers to account 0..255
X.<subscriber>.<subaccount>:<amount>   incoming amount >= positive threshold
X.<subscriber>.<start>-<end>           all transfers to an inclusive range
X.<subscriber>.<start>-<end>:<amount>  incoming amount >= threshold in that range
```

For range declarations the endpoints are inclusive and must satisfy `0 <= start < end <= 255`. Use the single-account form when both endpoints would be equal; invalid ranges are not reversed, clamped, or repaired. Integers use canonical decimal spelling without signs or leading zeroes.

An explicit threshold has at most two decimal places and a minimum of `0.01 ICP`. Omission means every incoming transfer to the numbered account or every account in the range. A global declaration means any transaction anywhere on the Ledger; it does not subscribe all of the canister's subaccounts. Range pokes contain only accounts that actually matched during the poll, never the entire declared range.

## Poke interpretation

A subscriber with a global declaration treats every poke as a reason to advance its global authoritative cursor. `poke([])` means the poll processed Ledger activity but no account declaration for that subscriber matched. A non-empty sorted vector identifies account declarations that matched and takes precedence over the empty global hint.

A subscriber without a global declaration receives only non-empty account hints. Any vector is a wake-up hint, not proof of payment or proof that no other Ledger activity occurred. Coalesce concurrent wake-ups and route them through the same reconciliation worker as an independent periodic timer.

Event Horizon reads the Ledger directly, so a poke can precede ICP Index visibility. The subscriber owns Index retry and backoff behavior. Event Horizon sends no transaction payload, retries, acknowledgements, or delivery guarantees.
