# Subscriber guide

Event Horizon accelerates an application's authoritative Ledger or optional Index reconciliation. Pokes are best-effort hints; exact grammar and semantics are normative in [`SPEC.md`](../SPEC.md).

## Security first: subscription funding is not subscriber consent

**A valid poke from a trusted Event Horizon canister is not proof that the
receiving canister wanted the subscription.** Any patron may fund a qualifying
declaration naming an arbitrary valid subscriber principal, without controlling
that canister or obtaining its knowledge or consent. This applies to account,
range, neuron and global subscriptions. Event Horizon admits the funded
declaration; it does not ask the subscriber to approve it.

For example, an application only cares about incoming ICP transfers to its
numeric subaccount `7`. An unrelated patron endows a subscription naming that
canister's subaccount `999` and arranges transfers into it. Event Horizon
correctly sends a poke identifying `subaccount 999`. A receiver checking its
local interest registry returns cheaply. A naïve zero-argument receiver has
discarded that hint and may unnecessarily start Ledger or Index reconciliation.
A patron can also finance a less restrictive threshold for an already-watched
target, or a global subscription the application never requested.

**Subscribers interested in particular accounts, ranges or neuron targets should
normally implement `poke(vec PokeMatch)` and inspect targets and amount hints
before initiating Ledger reads, Index queries or other consequential work.**
This cheap filtering is a central reason for the argument-bearing callback.
Maintain locally approved interests independently of Event Horizon's admitted
subscriptions. Caller authentication establishes the source, not local
interest or authorisation. Ignore an empty global-only hint unless global
activity is an intentional local policy.

## Integration flow

1. Identify the Event Horizon instance or instances for the ledgers the application needs.
2. Record each trusted Event Horizon canister principal and its immutable observed ledger.
3. For scoped interests, implement the recommended target-aware `poke(vec PokeMatch)` handler below. Use zero-argument `poke()` only under the [wake-up-only conditions](#wake-up-only-subscribers).
4. Authenticate `caller` and filter against locally approved per-Ledger interests before scheduling work.
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
type PokeTarget = variant {
  subaccount : nat64;
  neuron_nonce : nat64;
};

type PokeMatch = record {
  target : PokeTarget;
  max_amount : nat;
};

service : {
  poke : (vec PokeMatch) -> ();
}
```

Event Horizon has one outbound callback format and always sends the match
vector above. Payload-aware handling is the recommended default for scoped
subscribers; Event Horizon does not discover, negotiate, or retry another
signature. Candid permits a receiver with fewer arguments to ignore extra
arguments. See the [Candid argument and result sequence
rules](https://github.com/dfinity/candid/blob/master/spec/Candid.md#upgrading-and-subtyping).

Jupiter suffixes accept `<subscriber>`, `<subscriber>.<number>[:<amount>]`, `<subscriber>.<start>-<end>[:<amount>]`, and corresponding neuron forms using `n<nonce>` or `n<start>-<end>`. Numbers are canonical decimal `u64`. An inclusive range contains 2 through 256 targets and may begin anywhere. The complete routed memo still must fit 32 bytes.

Numeric subaccount `N` means subscriber ownership with `24 zero bytes || N.to_be_bytes()`. Neuron nonce `N` derives the staking subaccount with controller equal to the subscriber principal. ICP uses NNS Governance as owner; an SNS-token instance uses its verified SNS Governance. This excludes hotkeys, permission holders that are not the derivation controller, arbitrary governance owners, and arbitrary accounts.

`max_amount` is the largest individual qualifying incoming transfer Event Horizon observed for that target during the completed poll, in raw observed-token atomic units. It is a prefilter hint, not proof of payment. Specific matches take precedence over simultaneous global activity. `poke([])` means global-only activity. Non-empty targets are ordered as numeric subaccounts ascending and then neuron nonces ascending, with at most 256 retained targets per subscriber per poll. Any omission affects wake-up latency only.

Within each completed poll, Event Horizon attempts all globally matched
subscribers before specific-only subscribers, in ascending principal order
within each phase. A mixed match receives one poke in the global phase with
the specific payload. This fixed rule is independent of endowment amounts,
subscriber balances and polling cadence; there are no configurable priority
tiers. It orders outbound attempts only, without awaiting acknowledgement or
completion. Delivery and remote execution order between different canisters
remain unguaranteed, as does ordering across independent Event Horizon
instances. There is no retry or durable notification queue.

## Subscribing to multiple ledgers

One subscriber can trust and subscribe through multiple independent Event Horizon backend instances. Each instance has exactly one immutable observed-ledger context, so the ledger identity is supplied by the authenticated caller rather than repeated in every `PokeMatch`:

```text
Event Horizon caller A -> ICP Ledger
Event Horizon caller B -> SNS Foo Ledger
Event Horizon caller C -> IO Ledger
```

After authentication, `caller()` is the ledger namespace for a poke. Maintain a local trusted mapping conceptually like:

```text
Event Horizon caller principal
    -> observed ledger identity
    -> token decimals / local economic policy
    -> intentional global-interest flag
    -> locally approved subaccounts / neuron nonces
    -> local minimum atomic amount per approved target
    -> independent per-Ledger reconciliation cursor/state
```

This is the application's approved-interest configuration, not a copy of Event Horizon's admitted-subscription state. A match appearing in a poke never adds a locally authorised interest. Resolve the immutable observed-Ledger context from this registry without an Event Horizon subscription lookup or external authorisation query.

Never trust ledger identity or amount interpretation supplied by an untrusted caller. `subaccount 7` from the ICP caller and `subaccount 7` from an SNS caller are independent targets on different ledgers. Likewise, `neuron_nonce 7` means an NNS neuron controlled by the subscriber for the ICP caller, but that SNS's subscriber-controlled neuron for an SNS caller.

`max_amount` is caller-relative: it is ICP e8s for the ICP instance and the SNS token's raw atomic units for that SNS instance. This caller/instance context is why the callback does not repeat `observed_ledger` in every match.

## Safe multi-ledger handler

Use the argument-bearing handler as the normal pattern for scoped applications.
The lightweight sequence is:

1. Authenticate the caller against explicitly trusted Event Horizon instances.
2. Resolve its immutable observed-Ledger context from the local trusted registry.
3. Check whether the application intentionally accepts all activity for that Ledger.
4. Without local global interest, return immediately on an empty vector.
5. Discard non-empty target hints outside the locally approved interest set.
6. Compare `max_amount` with each target's local minimum in that Ledger's atomic units.
7. If nothing relevant remains, return without expensive work.
8. Otherwise coalesce the wake-up and schedule bounded authoritative reconciliation.
9. Make consequential decisions only from verified Ledger/Index data.
10. Keep independent periodic reconciliation through the same per-Ledger path.

Perform authentication and interest/amount checks **before** any asynchronous
Ledger, Index or other expensive inter-canister call. The checks are local;
they require no Event Horizon subscription lookup or external approval call.

```text
poke(matches):
    context = trusted_event_horizon_instances.get(caller)
    if context is absent:
        return

    # This flag is intentional LOCAL policy, not inferred from admission.
    # Specific hints replace the empty global hint when both match a poll,
    # so a global consumer accepts either vector shape as a wake-up.
    if context.wants_global_activity:
        schedule_coalesced_reconciliation(context.observed_ledger)
        return

    if matches is empty:
        return

    relevant = false
    for match in matches:
        policy = context.approved_targets.get(match.target)
        if policy is absent:
            continue
        if match.max_amount < policy.minimum_atomic_amount:
            continue
        relevant = true
        break

    if not relevant:
        return

    schedule_coalesced_reconciliation(context.observed_ledger)

# Scheduled work, not an inline Ledger/Index call from poke:
# - coalesces repeated wakes and prevents overlapping/redundant reconciliation
# - bounds and rate-limits work according to the application's own policy
# - reads authoritative Ledger/Index state using independent per-Ledger cursors
# - processes only locally authorised activity, not just the supplied hints
# - for Index-dependent work, shares one pending delayed follow-up after the
#   initial pass, where appropriate; see Index catch-up below
# The independent periodic timer schedules this same reconciliation path.
```

A non-empty vector does **not** mean there was no global activity. Specific
matches take precedence over the empty global-only hint. An intentionally global
consumer must therefore treat either shape as a global wake-up; a scoped
consumer must not process `poke([])` merely because its caller is trusted.

Reconciliation state must be independent per observed Ledger. Never advance a
cursor, accept a payment or perform accounting from a poke alone.

## Index catch-up

A global subscription can provide accelerated wake-ups to an Index canister
that supports `poke`. Applications may query that Index when they receive
their own specific-target pokes. Even with global-first dispatch, the
application can query before the Index has ingested the triggering Ledger
transaction. This is an eventual-consistency timing issue, not necessarily a
missed notification:

```text
Ledger records transaction
        |
Event Horizon completes poll
        |
        +--> Global poke attempted: Index
        |
        +--> Specific poke attempted: Application
                                        |
                                        +--> Immediate Index query
                                        |    Transaction may be absent
                                        |
                                        +--> Delayed Index query (~5s)
                                             Transaction may now be visible
```

For reconciliation that depends on an Index, the recommended sequence is:

1. Authenticate Event Horizon as caller and resolve its Ledger context.
2. Filter supplied targets and amount hints against local approved interests,
   cheaply rejecting unapproved targets, below-local-threshold amounts and
   unwanted global-only pokes.
3. Coalesce an accepted poke with any already pending reconciliation.
4. Perform the initial Index reconciliation with sensible work bounds.
5. After that first pass, schedule a second reconciliation approximately five
   seconds later where appropriate. Skip it if the application can reliably
   establish that the Index has already caught up sufficiently.
6. If the Index is still behind, retain pending work and use bounded retry/backoff
   or the application's independent periodic reconciliation.
7. Process transactions idempotently using authoritative transaction history
   and persistent local cursors.

Use a one-shot timer or equivalent scheduled task; return from `poke` promptly
instead of keeping it open while waiting. Five seconds is an illustrative
starting point, not an exact mandatory delay or protocol guarantee. An Index
may lag longer, and timer execution itself can be delayed. An empty initial
result or an unsuccessful five-second retry is not proof that no transaction
exists.

Multiple accepted notifications should share a pending follow-up; do not create
one timer and Index query per incoming poke or continually postpone the pending
check. Prevent concurrent or overlapping reconciliation across initial,
follow-up and periodic work. Bound and rate-limit that work and retain the
independent periodic timer. Schedule follow-ups only after the local interest
check, so unsolicited callbacks do not create a new cycles-drain opportunity.
Timers do not survive upgrades: restore needed scheduling from persistent work
and cursors as appropriate for the application.

Subscribers reading the Ledger directly do not need this Index-specific second
check. Each subscriber owns its retry policy; there is no new Event Horizon
callback setting. Event Horizon remains Index-independent: it never discovers,
queries, inspects or waits for an Index. See the current ICP guidance on
[message execution](https://docs.internetcomputer.org/references/message-execution-properties/)
and [timers](https://docs.internetcomputer.org/guides/backends/timers/).

## Local thresholds and poke poisoning

Another party can endow the same subscriber/target declaration with a less restrictive threshold. Event Horizon permanently keeps the least restrictive effective threshold for the watched account. For example:

```text
subscriber policy:             >= 0.01 ICP
later Event Horizon threshold: every transfer
```

After authenticating the caller and recognising the locally approved target,
the subscriber should return cheaply when `max_amount` is below its own `1_000_000` e8s requirement and reconcile when it is high enough. The local `0.01 ICP` minimum remains the subscriber's independent economic
policy even when the effective Event Horizon threshold becomes lower. This is
an intended reason `max_amount` exists, but it does not make the poke authoritative.

`max_amount` is the largest **individual** qualifying incoming transfer for that
target observed during the completed poll, not a sum. Use the observed Ledger's
atomic units and that Ledger's decimals/economic policy; never apply another
Ledger's amount scale. The hint is not a payment receipt, independent proof of
finality, or proof that an Index already exposes the transfer. Acceptance,
accounting and payment processing require verified authoritative reconciliation.
An Index may lag the Ledger; retry/backoff belongs to the subscriber.

Similarly, `poke([])` contains no specific target match. A subscriber uninterested in global activity can immediately return. Event Horizon provides no retries or acknowledgements, so independent periodic reconciliation remains required.


## Economics and bounded unwanted work

Unsolicited subscriptions require qualifying productive endowments. The
reference bases are 10 ICP for an account or neuron target, 20 ICP for a range,
and 1,000 ICP for global activity. Actual admission requirements remain dynamic
under the existing pricing formula; see [funding and permanent admission](#funding-and-permanent-admission).
These endowments impose an economic cost while contributing productive backing
to the shared operating model. They are not per-poke fees, and admission is permanent.

For a properly designed scoped receiver, an irrelevant authenticated callback
costs only a local caller/context lookup, target-interest checks and optional
amount comparisons, then a cheap return. Authentic Event Horizon callbacks
retain at most 256 specific targets per subscriber per poll, so this local scan
is bounded. Rejected hints must not start Ledger/Index reads or other expensive
work. Even relevant wakes should only schedule coalesced, bounded processing.

**The cost of funding unwanted subscriptions, combined with cheap recipient-side
rejection and coalesced processing, substantially limits the practical
cycles-drain opportunity.** This is defence in depth, not an authorisation
mechanism enforced by Event Horizon, and not an absolute economic guarantee.
An attacker may fund activity that genuinely meets the subscriber's approved
interests and amounts. Global consumers have different exposure because any
Ledger activity can be relevant. Keep reconciliation rate-limited or otherwise
bounded; never trigger disproportionately expensive operations directly per poke.

## Wake-up-only subscribers

Zero-argument `poke()` remains supported, but is an intentional general-wake
policy rather than the default shortcut for scoped applications:

```candid
service : {
  poke : () -> ();
}
```

Event Horizon still sends its standard `vec PokeMatch` argument. Candid permits
the zero-argument receiver to ignore that extra argument for both empty global
and non-empty specific-match payloads; no negotiation or alternate callback is
performed, and the receiver need not previously have exposed the longer signature.

This is appropriate only when the application deliberately treats **every
authenticated** Event Horizon poke as a general reconciliation wake-up, needs
no target/amount hints to decide relevance, and makes redundant wakes cheap and
bounded through coalescing, rate limiting or similar protection. It can suit
some indexing or general Ledger-monitoring applications. Authenticate the
caller, resolve its trusted Ledger context, schedule rather than reconcile
inline, and retain independent authoritative/periodic reconciliation.

Do not choose zero arguments merely for convenience when only a limited set of
accounts or neurons matters. Discarding the payload prevents cheap identification
of an unsolicited irrelevant target or a below-local-threshold hint. Choosing
`poke()` neither makes the application a global subscriber nor prevents a third
party from funding unwanted specific subscriptions. It is not inherently insecure;
safety depends on the intentionally bounded work performed after each authenticated
wake-up. Existing Index canisters are not assumed to export `poke`.
An Index that uses zero-argument `poke()` must still authenticate the caller
and bound/coalesce its indexing work; the interface does not establish catch-up
or completion before another subscriber executes.
