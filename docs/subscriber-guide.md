# Subscriber guide

Event Horizon accelerates an application's authoritative Ledger or optional Index reconciliation. Pokes are best-effort hints; exact grammar and semantics are normative in [`SPEC.md`](../SPEC.md).

## Integration flow

1. Identify the Event Horizon instance or instances for the ledgers the application needs.
2. Record each trusted Event Horizon canister principal and its immutable observed ledger.
3. Implement either the target-aware or wake-up-only `poke` callback.
4. Authenticate `caller` before interpreting any match.
5. Maintain authoritative reconciliation cursor/state independently per observed ledger.
6. Create the required Jupiter declaration for each instance.
7. Fund the exact declaration through that instance's reviewed Jupiter alias.
8. Use pokes only to accelerate authoritative reconciliation.
9. Retain an independent periodic reconciliation timer because pokes may be missed, delayed, duplicated, or rejected.

## Funding and permanent admission

Event Horizon prices the minimum qualifying cumulative productive endowment for
an exact declaration. Initial reference bases are 10 ICP for one account or
neuron target, 20 ICP for a range, and 1,000 ICP for a global Ledger
subscription. The 1,000 ICP figure is not a fixed price: current and frozen
requirements come from `get_pricing` and follow the existing monthly
`ceil(base × F / C)` policy with independent rounding.

A subscriber or any third-party patron sends ICP to Jupiter Faucet's configured
NNS neuron staking account with the complete `<alias>.<declaration>` memo. The
principal stays in Jupiter's productive staking path rather than entering Event
Horizon's liquid balance. A staking transfer does not immediately admit the
route. Event Horizon must later observe an authorised Faucet raw-ICP payout
whose preserved suffix is that declaration and receive complete Historian
evidence that the exact route's cumulative qualifying endowment meets the
current requirement.

The reviewed production Faucet has a seven-day stake-recognition delay, and
payout processing can add more time. Multiple endowments to the identical route
contribute cumulatively; cumulative principal is distinct from rewards already
distributed. Once admitted, the subscription is permanent. Extra endowment
does not increase priority or polling speed.

Subscription funding is always ICP-based, even when the selected Event Horizon
instance observes another token Ledger. The subscribed Ledger supplies activity
to match; Jupiter's raw-ICP payouts and Event Horizon's CMC conversion fund the
common operating cycles pool. The observed token and its transfer thresholds
remain independent of the ICP admission requirement.

## Callback and targets

```candid
type PokeTarget = variant { subaccount : nat64; neuron_nonce : nat64 };
type PokeMatch = record { target : PokeTarget; max_amount : nat };
service : { poke : (vec PokeMatch) -> (); }
```

Event Horizon has one outbound callback format and always sends the match
vector above. A subscriber chooses one of the two receiver styles described in
this guide; Event Horizon does not discover, negotiate, or retry another
signature. Candid permits a receiver with fewer arguments to ignore extra
arguments. See the [Candid argument and result sequence
rules](https://github.com/dfinity/candid/blob/master/spec/Candid.md#upgrading-and-subtyping).

Jupiter suffixes accept `<subscriber>`, `<subscriber>.<number>[:<amount>]`, `<subscriber>.<start>-<end>[:<amount>]`, and corresponding neuron forms using `n<nonce>` or `n<start>-<end>`. Numbers are canonical decimal `u64`. An inclusive range contains 2 through 256 targets and may begin anywhere. The complete routed memo still must fit 32 bytes.

Numeric subaccount `N` means subscriber ownership with `24 zero bytes || N.to_be_bytes()`. Neuron nonce `N` derives the staking subaccount with controller equal to the subscriber principal. ICP uses NNS Governance as owner; an SNS-token instance uses its verified SNS Governance. This excludes hotkeys, permission holders that are not the derivation controller, arbitrary governance owners, and arbitrary accounts.

`max_amount` is the largest individual qualifying incoming transfer Event Horizon observed for that target during the completed poll, in raw observed-token atomic units. It is a prefilter hint, not proof of payment. Specific matches take precedence over simultaneous global activity. `poke([])` means global-only activity. Non-empty targets are ordered as numeric subaccounts ascending and then neuron nonces ascending, with at most 256 retained targets per subscriber per poll. Any omission affects wake-up latency only.

## Wake-up-only subscribers

A consumer that needs only a wake-up signal may expose this alternative
receiver interface:

```candid
service : { poke : () -> (); }
```

Event Horizon still sends its standard match-vector payload. A wake-up-only
subscriber may expose a zero-argument `poke()` update method; the extra
argument is ignored. The subscriber can use that notification to schedule
authoritative reconciliation without interpreting target or amount hints.
This works for both empty global payloads and nonempty specific-match payloads;
it does not depend on the canister previously exposing the longer signature.

Choosing `poke()` does not make a subscription global. Global, account, range,
and neuron declarations retain their existing meanings. The zero-argument
receiver simply discards the targets and `max_amount` values, so it must not
claim to have performed the payload-aware local filtering described below.

Authenticate the caller before scheduling meaningful work. A multi-ledger
consumer must retain a trusted Event-Horizon-caller-to-observed-ledger mapping
and independent reconciliation state for every ledger. Treat each callback as
a best-effort hint, coalesce repeated wake-ups locally, prevent overlapping
reconciliation work, and retain an independent periodic reconciliation path.

For example, an index canister that implements the callback could use the
following pseudocode; this does not imply that every existing Index canister
already exports `poke`:

```text
poke():
    context = trusted_event_horizon_instances.get(caller)
    if context is absent:
        return

    mark reconciliation needed for context.observed_ledger
    schedule reconciliation unless already scheduled or running
```

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
