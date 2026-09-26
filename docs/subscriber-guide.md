# Subscriber guide

`poke : (vec nat8) -> ()` is unchanged. Values identify numbered subaccounts on the selected observed token ledger. Thresholds use that token's verified decimals; the Jupiter endowment and displayed subscription price remain ICP. A poke may precede an optional Index, and subscriber reconciliation remains authoritative.

Implement and authenticate this endpoint:

```candid
service : { poke : (vec nat8) -> (); }
```

## Declaration forms

```text
<alias>.<subscriber>                         global Ledger trigger
<alias>.<subscriber>.<subaccount>            all incoming transfers to account 0..255
<alias>.<subscriber>.<subaccount>:<amount>   incoming amount >= positive observed-token threshold
<alias>.<subscriber>.<start>-<end>           all transfers to an inclusive range
<alias>.<subscriber>.<start>-<end>:<amount>  incoming amount >= observed-token threshold in that range
```

Canonical ICP uses alias `X`. Planned IO has intended alias `I`, which is not
claimed to be published. A future reviewed alias need not be one character.
The complete Jupiter memo—including alias, dot, subscriber, scope or range,
and optional threshold—must fit the 32-byte limit.

For range declarations the endpoints are inclusive and must satisfy `0 <= start < end <= 255`. Use the single-account form when both endpoints would be equal; invalid ranges are not reversed, clamped, or repaired. Integers use canonical decimal spelling without signs or leading zeroes.

An explicit threshold uses at most the observed ledger's verified decimals and may be one raw token unit. Omission means every incoming transfer to the numbered account or every account in the range. A global declaration means any block anywhere on the observed Ledger; it does not subscribe all of the canister's subaccounts.

## Poke interpretation

A subscriber with a global declaration treats every poke as a reason to advance its global authoritative cursor. `poke([])` means the poll processed Ledger activity but no account declaration for that subscriber matched. A non-empty sorted vector identifies account declarations that matched and takes precedence over the empty global hint.

A subscriber without a global declaration receives only non-empty account hints. Any vector is a wake-up hint, not proof of payment or proof that no other Ledger activity occurred. Coalesce concurrent wake-ups and route them through the same reconciliation worker as an independent periodic timer.

Event Horizon reads the selected Ledger directly (legacy `query_blocks` for canonical ICP and ICRC-3 for non-ICP ledgers), so a poke can precede an optional application Index. The subscriber owns any Index retry and backoff behavior. Event Horizon sends no transaction payload, retries, acknowledgements, or delivery guarantees.
