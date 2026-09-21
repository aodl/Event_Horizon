# Subscriber guide

Implement this endpoint:

```candid
service : { poke : (vec nat8) -> (); }
```

The vector contains the unique numbered subaccounts on your canister that Event Horizon saw relevant activity for during one completed poll. Event Horizon coalesces all matches for your canister into a single call and sends the subaccount numbers in deterministic ascending order.

Authenticate the Event Horizon caller, validate/recognise the supplied subaccount numbers, coalesce the wake-up with your existing reconciliation worker, and return promptly. Do not treat a poke or its subaccount list as proof of a payment or as authoritative transaction data.

Your own periodic reconciliation remains mandatory. It is the correctness path.

## Optional threshold

A subscription memo may omit the amount entirely:

```text
X.<subscriber>.<subaccount>
```

That means every incoming transfer to that watched account is relevant.

If an amount is supplied:

```text
X.<subscriber>.<subaccount>:<amount>
```

matching is inclusive (`>=`) and the smallest explicit threshold is `0.01 ICP`.

## Event Horizon may beat the Index

Event Horizon reads the ICP Ledger directly. You may therefore receive a poke before the ICP Index exposes the triggering transaction. This is a useful latency property, not an error. If your reconciliation uses the Index and sees nothing new, choose the recheck/backoff behaviour appropriate to your application.
