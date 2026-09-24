# Event Horizon v1 protocol specification

This file is the normative design contract for Event Horizon v1. Implementation complexity that is not required by this specification should not be introduced without an explicit design change.

## 1. Purpose

Event Horizon improves the responsiveness of canisters that react to incoming ICP without becoming part of their correctness path.

Event Horizon reads the live ICP Ledger and makes a best-effort call to `poke : (vec nat8) -> ()` on subscribed canisters when relevant transfers are observed. A non-empty poke contains the distinct numbered subaccounts that matched during that poll. An empty poke signals activity for a global Ledger subscription. Neither form contains transaction identifiers, amounts, memos, senders, or other transaction data. Each subscriber owns its authoritative reconciliation cursor and must retain independent periodic reconciliation.

A missed, rejected, delayed, or duplicated poke must therefore affect only latency.

## 2. Product and trust boundary

Event Horizon is a standalone downstream product powered by Jupiter Faucet endowments.

The Event Horizon backend:

- has exactly one production application query, `get_pricing`, containing no subscription or administrative data;
- is intended to be observed in production while controlled, then made immutable by setting its controller list to empty;
- uses native public canister status and public canister logs for operational observability;
- is independently funded and operational after subscriptions have been admitted.

Jupiter Historian is used for admission verification. Jupiter Faucet and Disburser do not depend on Event Horizon and do not consume its pokes.

## 3. Subscription declaration

A Jupiter Faucet endowment uses exactly one of these full memo forms:

```text
X.<compact-subscriber-principal>
X.<compact-subscriber-principal>.<numbered-subaccount>
X.<compact-subscriber-principal>.<numbered-subaccount>:<minimum-ICP>
X.<compact-subscriber-principal>.<start-subaccount>-<end-subaccount>
X.<compact-subscriber-principal>.<start-subaccount>-<end-subaccount>:<minimum-ICP>
```

Examples:

```text
X.r5m5ydiaaaaaaaaqanaacai
X.r5m5ydiaaaaaaaaqanaacai.7
X.r5m5ydiaaaaaaaaqanaacai.7:0.01
```

`X` is a Jupiter Faucet runtime alias whose mapping is owned by Jupiter Historian. Event Horizon itself receives only the outgoing suffix, for example:

```text
r5m5ydiaaaaaaaaqanaacai
r5m5ydiaaaaaaaaqanaacai.7
r5m5ydiaaaaaaaaqanaacai.7:0.01
r5m5ydiaaaaaaaaqanaacai.7-18
r5m5ydiaaaaaaaaqanaacai.7-18:0.01
```

The suffix grammar is:

```text
<principal>
<principal> "." ( <subaccount> | <start> "-" <end> ) [ ":" <amount> ]
```

Rules:

- principals may be the canonical hyphenated representation or the compact representation with group separators removed;
- a principal alone declares a global Ledger subscription: any transaction processed in a completed poll is relevant;
- a global declaration does not represent all 256 subaccounts of the subscriber;
- anonymous and management principals are invalid;
- subaccount is a decimal integer `0..255`;
- integer spelling is canonical decimal: `0` or a nonzero digit followed by decimal digits; signs and leading zeroes are invalid;
- range endpoints are inclusive and must satisfy `0 <= start < end <= 255`;
- reverse, degenerate, out-of-range, or malformed ranges are rejected and never normalized;
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

An admitted range is expanded once into the existing watched-account map, deriving one account identifier for every inclusive endpoint value. Expansion is bounded to 256 entries. There is no runtime range scanner or persistent range registry. Overlapping single and range declarations merge by the same lowest-threshold rule. Since admission is permanent, an effective threshold can only remain unchanged or become less restrictive.

## 5. Admission

A declaration is eligible for permanent admission only after both are true:

1. Event Horizon observes an authorised Jupiter Faucet raw-ICP payout to Event Horizon carrying that declaration in its ICRC-1 memo.
2. Jupiter Historian reports a cumulative qualifying Jupiter Faucet endowment for that exact raw-ICP route at least equal to the current account, range, or global admission price, and reports the result as complete/trustworthy.

The approved mainnet Faucet payout source is fixed in the production Wasm.

Admission is permanent. There is no expiry, deletion, subscriber balance, subscriber quota, priority tier, or administration interface.

Global, range, and account declarations are distinct exact routes with class-specific prices. A range has one price regardless of width. The price in force when Event Horizon evaluates the declaration is authoritative. A declaration below that price remains unadmitted; later Faucet payouts may raise the exact-route total above the then-current requirement. Additional value for an admitted declaration creates no duplicate, priority, faster polling, or additional poke.

If Historian is unavailable/incomplete for one payout, Event Horizon does not persist an admission-retry queue. A later perpetual Faucet payout carrying the same declaration provides a natural later admission opportunity.

## 6. Ledger reader

Event Horizon reads the ICP Ledger directly; it does not depend on the ICP Index.

A fresh installation is prospective. On first successful Ledger observation it records the current chain tip and processes future activity only.

Each poll captures a fixed exclusive ending boundary from the Ledger chain length. The poll processes only the interval from its durable cursor to that boundary. Blocks arriving while the poll is executing belong to a later poll.

For each relevant transfer to an admitted watched account, Event Horizon inserts that account's numbered subaccount into a transient sorted set keyed by the subscriber principal. It separately records whether the poll processed any transaction. After the fixed boundary has been completely processed, it incorporates admitted global subscribers once when that flag is true. It does not enumerate global subscribers per transaction.

Event Horizon attempts at most one poke to each accumulated subscriber. A non-empty sorted unique account set takes precedence even when the same subscriber also matched globally. A subscriber with only a global match receives `poke([])`. A poll that processes no transactions produces no global poke.

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

The vector contains the distinct numbered subaccounts on that subscriber canister that saw relevant activity in the completed poll. Event Horizon sends at most one poke per subscriber per completed poll, and a non-empty vector is deterministically sorted and deduplicated. For an admitted global subscriber, `poke([])` means that the poll processed Ledger activity without a more-specific account match. Any poke tells a global subscriber to advance its global authoritative cursor. A non-empty vector additionally identifies account declarations that matched.

The subaccount list is a wake-up hint only. It is not proof that a payment exists, contains no authoritative transaction data, and is not proof that no other Ledger activity occurred. The subscriber remains responsible for its own global and account Ledger/Index cursors and independent periodic reconciliation.

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

Account, range, and global endowments enter the same pool. Their different admission prices do not create subscriber-specific cycle allowances.

Larger endowments do not buy priority or lower latency.

Raw ICP accumulated in Event Horizon is periodically classified once and moved by one serialized funding state machine. The retained leg uses the ICP Ledger's legacy `transfer` method to the CMC account identifier derived from Event Horizon's principal, with the standard top-up memo, followed by `notify_top_up(block_index)`. Value-moving Ledger calls use guaranteed-response (unbounded-wait) semantics while their deterministic identities remain persisted. The CMC leg deliberately does not use `icrc1_transfer`.

Production surplus diversion is enabled only when the immutable compile-time `SURPLUS_CANISTER` is `Some(principal)`. It is currently `None`. Disabled operation resets the policy to level zero, creates no entitlement, and preserves the single-transfer CMC behavior. There is no runtime setter or surplus API. When enabled, the receiver is the designated canister's default all-zero-subaccount ICP account; Event Horizon calls no method on it and has no responsibility for its later treasury or governance behavior.

The hourly funding lane records the lowest **observed** liquid-cycles balance in fixed seven-day epochs. The first observation after enabling starts the first epoch at level 0. An observed minimum at least 150 T raises the persistent level by one; a minimum from 100 T inclusive to 150 T exclusive lowers it by one; a minimum below 100 T resets it to zero. Levels are bounded to `0..19`, and each level represents five percentage points, so diversion rises by at most 5 points per epoch and never exceeds 95%. Processing a long gap applies at most one transition and starts a fresh epoch from the current observation. No historical epochs, burn-rate accounting, or additional timer exist.

At planning time, a current liquid balance below 150 T forces effective diversion to zero without immediately changing the stored level. For raw balance `B`, current Ledger fee `F`, and effective percentage `P`, `P = 0` retains the existing `B - F` CMC transfer. An active split requires `B > 2F`, sets `allocatable = B - 2F`, computes `surplus = floor(allocatable * P / 100)` with overflow-safe integer arithmetic, and assigns the remainder to retained funding. Zero/dust legs fall back to the single CMC transfer. Thus rounding favors Event Horizon and even level 19 intentionally retains approximately 5% of fee-net funding.

The complete plan is persisted before value moves, including the surplus destination account and fixed memo. The retained transfer and successful CMC notification must finish before the planned surplus transfer identity receives its deterministic creation time and may be submitted. Configuration changes affect later plans only; every pending plan continues with its frozen destination and memo. Immediately before that surplus transfer, liquid cycles must still be at least 150 T; otherwise the surplus leg is cancelled and its ICP remains for a later conservative plan. ICP arriving during a pending plan is outside that plan. CMC refund or terminal failure cancels its planned surplus.

The surplus transfer uses legacy Ledger transfer recovery to the frozen default account with memo `6004796182999946033` (big-endian ASCII `SURPLUS1`). A pending transfer is self-contained: destination account identifier, memo, amount, fee, and creation time are all stable. Acceptance or duplicate recovery completes the plan. Uncertain outcomes retain the exact identity; definite no-debit outcomes clear it for later replanning; `TxTooOld` logs `SURPLUS_TRANSFER_IDENTITY_EXPIRED`, clears it, and preserves autonomous liveness. There is no transaction-history recovery, administrative repair, or second money-moving worker.

Only the minimum durable financial state necessary to avoid duplicate/stranded value is retained. One durable `FundingState` at stable-memory ID 7 atomically describes the next retained CMC transfer, CMC notification, or surplus transfer action and initializes directly to `Idle` on first installation. A split plan freezes the complete future surplus destination, memo, amount, and fee before the retained transfer begins.

## 10. Dynamic admission pricing

The CMC's integer `xdr_permyriad_per_icp` is the sole pricing input. Event Horizon stores at most one successful observation per UTC day and makes roughly one ordinary observation attempt per day. Before issuing the nonessential CMC query it requires the current liquid cycles balance to cover both `RESERVE_PROTECTION_CYCLES` and the query's current `Call::get_cost()`. A day skipped to protect the reserve is consumed as that day's attempt. Failed, skipped, or missed days cause no retry loop, alternate oracle, or backfill.

The history window contains the current UTC day bucket and the preceding 1,460 UTC day buckets, for at most 1,461 observations. Expired buckets are discarded at pricing maintenance. The first successful observation after a fresh install or an upgrade without pricing state becomes the initial latest rate and rolling floor; initial prices are therefore 10 ICP for an account declaration, 20 ICP for a range declaration, and 100 ICP for a global declaration.

For retained floor `F` and selected latest rate `C`, prices are calculated independently with overflow-safe integer arithmetic:

```text
account_icp = ceil(10 × F / C)
range_icp   = ceil(20 × F / C)
global_icp  = ceil(100 × F / C)
```

Prices are whole ICP and each economic basis is rounded independently. The range result is not twice the rounded account result. Because `ceil(20F/C) = ceil(ceil(100F/C)/5)`, the public/current range value is derived with overflow-safe ceiling division from the durably stored global value. This leaves the stable internal account/global `Price` encoding unchanged.

Prices may become effective only at 00:00:00 UTC on the first day of a month. The next price is frozen exactly seven 24-hour days before that timestamp. Only observations recorded strictly before the freeze timestamp participate in that calculation; later observations can affect a subsequent epoch. A frozen price is immutable. If the latest participating CMC rate timestamp is more than seven 24-hour days old at freeze, the current prices are carried forward and marked stale rather than recalculated. At the effective timestamp the frozen price becomes current.

`get_pricing : () -> (Pricing) query` is read-only and tightly bounded. It returns account, range, and global current and optional frozen prices, initialization state, effective and freeze timestamps, retained floor and latest observations with timestamps, and the stale carry-forward flag. It returns no subscriptions, status, logs, or administration controls.

## 11. Global polling cadence

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

## 12. Production observability and immutability

The production backend exposes only the `get_pricing` application query. The release audit rejects every other application query, composite query, or update.

Before controller removal:

- set `status_visibility = public`;
- set `log_visibility = public`;
- use a deliberately small rolling log and emit exceptional transitions only;
- verify the reproducible production Wasm hash and mainnet constants;
- observe real cycles burn, crawl liveness, funding conversion and subscriber behaviour.

Immutability means an empty controller list, not transfer to a blackhole canister.

## 13. Frontend

The frontend is a separately controlled Rust canister serving certified HTTP assets embedded into its Wasm. Its module hash therefore commits to both serving logic and frontend assets.

The mutable frontend's certified embedded JavaScript discovers the backend canister through the deployment environment and directly issues the read-only `get_pricing` query. No frontend update call is involved. It displays authoritative account/range/global current and frozen prices, exact UTC timestamps, floor/latest observations, and stale carry-forward state. Its builder exposes global, single-account, and inclusive-range modes, validates without repairing input, and continuously reports complete memo bytes against the 32-byte limit. The exact CMC integer rates are formatted for display with four decimal places as XDR/ICP. It recommends a frozen higher upcoming requirement because Faucet payout and admission are delayed; it does not recommend a lower frozen price before that price becomes effective. Backend admission remains authoritative.
