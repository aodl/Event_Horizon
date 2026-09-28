# Subscriber guide

Event Horizon accelerates an application's authoritative Ledger or optional Index reconciliation. Pokes are best-effort hints; exact grammar and semantics are normative in [`SPEC.md`](../SPEC.md).

## Integration flow

1. Identify the Event Horizon instance or instances for the ledgers the application needs.
2. Record each trusted Event Horizon canister principal and its immutable observed ledger.
3. Implement the target-aware `poke` callback.
4. Authenticate `caller` before interpreting any match.
5. Maintain authoritative reconciliation cursor/state independently per observed ledger.
6. Create the required Jupiter declaration for each instance.
7. Fund the exact declaration through that instance's reviewed Jupiter alias.
8. Use pokes only to accelerate authoritative reconciliation.
9. Retain an independent periodic reconciliation timer because pokes may be missed, delayed, duplicated, or rejected.

## Callback and targets

```candid
type PokeTarget = variant { subaccount : nat64; neuron_nonce : nat64 };
type PokeMatch = record { target : PokeTarget; max_amount : nat };
service : { poke : (vec PokeMatch) -> (); }
```

Jupiter suffixes accept `<subscriber>`, `<subscriber>.<number>[:<amount>]`, `<subscriber>.<start>-<end>[:<amount>]`, and corresponding neuron forms using `n<nonce>` or `n<start>-<end>`. Numbers are canonical decimal `u64`. An inclusive range contains 2 through 256 targets and may begin anywhere. The complete routed memo still must fit 32 bytes.

Numeric subaccount `N` means subscriber ownership with `24 zero bytes || N.to_be_bytes()`. Neuron nonce `N` derives the staking subaccount with controller equal to the subscriber principal. ICP uses NNS Governance as owner; an SNS-token instance uses its verified SNS Governance. This excludes hotkeys, permission holders that are not the derivation controller, arbitrary governance owners, and arbitrary accounts.

`max_amount` is the largest individual qualifying incoming transfer Event Horizon observed for that target during the completed poll, in raw observed-token atomic units. It is a prefilter hint, not proof of payment. Specific matches take precedence over simultaneous global activity. `poke([])` means global-only activity. Non-empty targets are ordered as numeric subaccounts ascending and then neuron nonces ascending, with at most 256 retained targets per subscriber per poll. Any omission affects wake-up latency only.

## Subscribing to multiple ledgers

One subscriber can trust and subscribe through multiple independent Event Horizon backend instances. Each instance has exactly one immutable observed-ledger context, so the ledger identity is supplied by the authenticated caller rather than repeated in every `PokeMatch`:

```text
Event Horizon caller A -> ICP Ledger
Event Horizon caller B -> SNS Foo Ledger
Event Horizon caller C -> IO Ledger
```

After authentication, `caller()` is the ledger namespace for a poke. Maintain a trusted mapping conceptually like:

```text
Event Horizon caller principal
    -> observed ledger identity
    -> token decimals / local economic policy
    -> authoritative reconciliation cursor/state
```

Never trust ledger identity or amount interpretation supplied by an untrusted caller. `subaccount 7` from the ICP caller and `subaccount 7` from an SNS caller are independent targets on different ledgers. Likewise, `neuron_nonce 7` means an NNS neuron controlled by the subscriber for the ICP caller, but that SNS's subscriber-controlled neuron for an SNS caller.

`max_amount` is caller-relative: it is ICP e8s for the ICP instance and the SNS token's raw atomic units for that SNS instance. This caller/instance context is why the callback does not repeat `observed_ledger` in every match.

## Safe multi-ledger handler

```text
poke(matches):
    context = trusted_event_horizon_instances.get(caller)
    if context is absent:
        return

    if matches is empty:
        if context.global_activity_not_relevant:
            return
        reconcile context.observed_ledger
        return

    relevant = []
    for match in matches:
        if target not recognised for this ledger context:
            continue
        if match.max_amount < local_threshold(context, match.target):
            continue
        relevant.push(match.target)

    if relevant is empty:
        return

    reconcile authoritative state for context.observed_ledger
    perform consequential work only from reconciled ledger state
```

Reconciliation cursor/state normally must be independent for each observed ledger; one universal cursor cannot reconcile unrelated ledgers.

## Local thresholds and poke poisoning

Another party can endow the same subscriber/target declaration with a less restrictive threshold. Event Horizon permanently keeps the least restrictive effective threshold for the watched account. For example:

```text
subscriber policy:             >= 0.01 ICP
later Event Horizon threshold: every transfer
```

After authenticating the caller, the subscriber can return cheaply when `max_amount` is below its own `1_000_000` e8s requirement and reconcile when it is high enough. This is an intended reason `max_amount` exists, but it does not make the poke authoritative. The subscriber must make consequential decisions only from reconciled Ledger or Index state.

Similarly, `poke([])` contains no specific target match. A subscriber uninterested in global activity can immediately return. Event Horizon provides no retries or acknowledgements, so independent periodic reconciliation remains required.
