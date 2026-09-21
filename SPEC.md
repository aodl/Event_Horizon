# Event Horizon v1 protocol specification

This file is the normative design contract for Event Horizon v1. Implementation complexity that is not required by this specification should not be introduced without an explicit design change.

## 1. Purpose

Event Horizon improves the responsiveness of canisters that react to incoming ICP without becoming part of their correctness path.

Event Horizon reads the live ICP Ledger and makes a best-effort call to `poke : (vec nat8) -> ()` on subscribed canisters when relevant transfers are observed. The poke contains only the distinct numbered subaccounts that matched during that poll; it contains no transaction identifiers, amounts, memos, senders, or other transaction data. Each subscriber owns its authoritative reconciliation cursor and must retain independent periodic reconciliation.

A missed, rejected, delayed, or duplicated poke must therefore affect only latency.

## 2. Product and trust boundary

Event Horizon is a standalone downstream product powered by Jupiter Faucet endowments.

The Event Horizon backend:

- has no production application API;
- is intended to be observed in production while controlled, then made immutable by setting its controller list to empty;
- uses native public canister status and public canister logs for operational observability;
- is independently funded and operational after subscriptions have been admitted.

Jupiter Historian is used for admission verification. Jupiter Faucet and Disburser do not depend on Event Horizon and do not consume its pokes.

## 3. Subscription declaration

A Jupiter Faucet endowment uses either of these full memo forms:

```text
X.<compact-subscriber-principal>.<numbered-subaccount>
X.<compact-subscriber-principal>.<numbered-subaccount>:<minimum-ICP>
```

Examples:

```text
X.r5m5ydiaaaaaaaaqanaacai.7
X.r5m5ydiaaaaaaaaqanaacai.7:0.01
```

`X` is a Jupiter Faucet runtime alias whose mapping is owned by Jupiter Historian. Event Horizon itself receives only the outgoing suffix, for example:

```text
r5m5ydiaaaaaaaaqanaacai.7
r5m5ydiaaaaaaaaqanaacai.7:0.01
```

The suffix grammar is:

```text
<principal> "." <subaccount> [ ":" <amount> ]
```

Rules:

- principals may be the canonical hyphenated representation or the compact representation with group separators removed;
- anonymous and management principals are invalid;
- subaccount is a decimal integer `0..255`;
- omitting `:<amount>` means **every incoming transfer** to that watched account is relevant;
- when present, amount is decimal ICP with one or two fractional digits, or an integer amount;
- fractional amounts require a leading zero (`0.1`, not `.1`);
- no sign, exponent or comparison operator is accepted;
- explicit threshold matching semantics are always `transfer amount >= declared amount`;
- the smallest valid explicit amount is `0.01 ICP`;
- a colon with no amount is invalid; omission means omitting the colon and amount together;
- the complete Jupiter Faucet memo must fit the Ledger's 32-byte memo limit.

Internally all values are integer e8s. Floating-point arithmetic is forbidden. The implementation may use `0 e8s` as an internal sentinel for an omitted threshold because an explicitly declared threshold can never be below `0.01 ICP`.

## 4. Watched account

The numbered subaccount convention is 32 zero bytes with the integer stored in the last byte. Subaccount `0` is therefore the default all-zero subaccount.

The watched legacy ICP account identifier is derived from the subscriber principal and that 32-byte subaccount using the standard `\x0Aaccount-id` SHA-224 + CRC-32 construction.

At most one effective subscription is required for a watched account. If another admitted declaration for the same account has a less restrictive threshold, the stored threshold becomes the less restrictive value. An unfiltered declaration (no threshold) therefore subsumes every thresholded declaration for that same account.

## 5. Admission

A declaration is eligible for permanent admission only after both are true:

1. Event Horizon observes an authorised Jupiter Faucet raw-ICP payout to Event Horizon carrying that declaration in its ICRC-1 memo.
2. Jupiter Historian reports at least **10 ICP** cumulative qualifying Jupiter Faucet endowment for that exact raw-ICP route and reports the result as complete/trustworthy.

The approved mainnet Faucet payout source is fixed in the production Wasm.

Admission is permanent. There is no expiry, deletion, subscriber balance, subscriber quota, priority tier, or administration interface.

If Historian is unavailable/incomplete for one payout, Event Horizon does not persist an admission-retry queue. A later perpetual Faucet payout carrying the same declaration provides a natural later admission opportunity.

## 6. Ledger reader

Event Horizon reads the ICP Ledger directly; it does not depend on the ICP Index.

A fresh installation is prospective. On first successful Ledger observation it records the current chain tip and processes future activity only.

Each poll captures a fixed exclusive ending boundary from the Ledger chain length. The poll processes only the interval from its durable cursor to that boundary. Blocks arriving while the poll is executing belong to a later poll.

For each relevant transfer to an admitted watched account, Event Horizon inserts that account's numbered subaccount into a transient sorted set keyed by the subscriber principal. After the fixed boundary has been completely processed, Event Horizon attempts at most one poke to each accumulated subscriber, carrying the sorted unique subaccount numbers that matched during that poll.

## 7. Archives and history gaps

Event Horizon does not traverse Ledger archives.

If the next required block is no longer locally available, Event Horizon:

1. writes one concise public `HISTORY_GAP` exceptional log for the skipped interval;
2. advances its cursor to the first locally available block;
3. continues normal operation.

It does not halt, reconstruct the gap, or create administrative recovery work.

This is acceptable because subscribers retain authoritative reconciliation.

## 8. Poke contract

Subscriber contract:

```candid
service : {
  poke : (vec nat8) -> ();
}
```

The vector contains the distinct numbered subaccounts on that subscriber canister that saw relevant activity in the completed poll. Event Horizon sends at most one poke per subscriber per completed poll, and the vector is deterministically sorted and deduplicated.

The subaccount list is a wake-up hint only. It is not proof that a payment exists and it does not contain authoritative transaction data. The subscriber remains responsible for reading its own Ledger/Index history for each indicated subaccount and for retaining independent periodic reconciliation.

There is:

- no block index, transaction amount, memo, sender, or transaction payload;
- no acknowledgement processing;
- no retry;
- no durable notification queue;
- no delivery journal;
- no ordering or delivery guarantee.

Subscribers should authenticate Event Horizon as caller, validate/recognise the supplied subaccount numbers, coalesce concurrent wake-ups, and route both pokes and their independent periodic timer through the same authoritative reconciliation path.

Because Event Horizon reads the Ledger directly, a poke may arrive before the ICP Index exposes the triggering transaction. The subscriber owns any Index retry/backoff policy.

## 9. Pooled funding

All Event Horizon cycles are a common operating pool. There is no attribution of cycles expenditure to individual subscriptions.

The working subscription requirement is **10 ICP** qualifying Jupiter Faucet endowment per exact declaration. A separate developer baseline endowment (planned at approximately 100 ICP) supplies common infrastructure funding.

Larger endowments do not buy priority or lower latency.

Raw ICP accumulated in Event Horizon is periodically converted to cycles. The conversion uses the ICP Ledger's legacy `transfer` method to the CMC account identifier derived from Event Horizon's principal, with the standard top-up memo, followed by `notify_top_up(block_index)`. It deliberately does not use `icrc1_transfer` for the CMC leg.

Only the minimum durable financial state necessary to avoid duplicate/stranded value is retained: an idle state, a deterministic legacy-transfer identity awaiting a definitive Ledger block, or an accepted block awaiting CMC notification. Transport ambiguity reuses the same transfer identity; explicit CMC refunds clear naturally; there is no administrative recovery API or financial journal.

## 10. Global polling cadence

`T = 10^12 cycles`.

| Mode | Added delay after completed poll | Enter at | Exit below |
|---|---:|---:|---:|
| Reserve protection | ordinary polls suspended | — | 1 T |
| Economy | 1 hour | 2 T | 1 T |
| Standard | 10 minutes | 5 T | 3 T |
| Fast | 2 minutes | 10 T | 6 T |
| Very fast | 10 seconds | 25 T | 15 T |
| Continuous | 0 | 100 T | 60 T |

Hysteresis is stateful. A balance increase may jump directly to the fastest mode whose entry threshold is met. A balance decrease retains the current mode until its exit threshold is crossed, then falls back as far as necessary.

Continuous means no deliberately inserted delay after one complete poll; polls never overlap and execution must yield between bounded asynchronous operations.

## 11. Production observability and immutability

The production backend exposes no callable application methods. Its Candid service is empty.

Before controller removal:

- set `status_visibility = public`;
- set `log_visibility = public`;
- use a deliberately small rolling log and emit exceptional transitions only;
- verify the reproducible production Wasm hash and mainnet constants;
- observe real cycles burn, crawl liveness, funding conversion and subscriber behaviour.

Immutability means an empty controller list, not transfer to a blackhole canister.

## 12. Frontend

The frontend is a separately controlled Rust canister serving certified HTTP assets embedded into its Wasm. Its module hash therefore commits to both serving logic and frontend assets.

The frontend is informational. It has no Event Horizon backend application API to call. It may use Jupiter Historian to explain/verify the corresponding endowment route.
