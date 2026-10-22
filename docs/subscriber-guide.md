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
local interest registry returns cheaply. A receiver that discards the payload has
discarded that hint and may unnecessarily start Ledger or Index reconciliation.
A patron can also finance a less restrictive threshold for an already-watched
target, or a global subscription the application never requested.

**Subscribers interested in particular accounts, ranges or neuron targets should
normally implement the target-aware `poke` handler below and inspect targets and amount hints
before initiating Ledger reads, Index queries or other consequential work.**
This cheap filtering is a central reason for the argument-bearing callback.
Maintain locally approved interests independently of Event Horizon's admitted
subscriptions. Caller authentication establishes the source, not local
interest or authorisation. Ignore an empty global-only hint unless global
activity is an intentional local policy.

## Integration flow

1. Identify the Event Horizon instance or instances for the ledgers the application needs.
2. Record each trusted Event Horizon canister principal and its immutable observed ledger.
3. Implement the unified `poke(PokeNotification)` handler below, retaining subject and amount hints even for self subscriptions.
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

## Subscriber, subject and permanent aliases

The subscriber receives notifications; the subject owns the watched numbered
subaccount, or supplies the controller principal for neuron staking-account
derivation. Omitting the subject selects the subscriber itself. Identity does
not grant control or establish consent.

The complete Jupiter memo uses payment routing prefix `E.` for the canonical
ICP instance. `X` is retired. The suffix grammar is:

```text
<subscriber>
<subscriber>.<target>[:<amount>]
<subscriber>@<subject>.<target>[:<amount>]
```

Each reference accepts a canonical or compact canister principal, or a cached
permanent alias matching `[A-Z][A-Z0-9]{0,7}`. Currently Historian publishes only
`E` → `eo6ei-gaaaa-aaaar-qchra-cai`. Other letters below are hypothetical:
`E.A@B.7` notifies A about B's subaccount 7; `E.B@A.7` reverses the relationship.
`E.A.7` and `E.A@A.7` identify the same self interest. Alias/principal spellings
resolve by principal bytes. `E.A@B` is invalid: global subscriptions have no subject.

A target is a canonical unsigned decimal `u64`, including zero, or an inclusive
range with 2–256 targets. Prefix `n` selects neuron nonces, for example
`E.A@B.n0-2:0.01`. The subject is the derivation controller, **not** the Governance
owner or a neuron ID. The owner remains NNS Governance for ICP or verified SNS
Governance for an applicable token. All complete memos must fit 32 ASCII bytes.
Threshold precision and units come from the observed token, not admission ICP.

Historian publishes permanent additive identity mappings separately from Faucet
payment routing: only E currently has special payout-routing meaning. Each
instance attempts refresh after install and once per UTC day, persists its
accepted cache across upgrades, and validates the whole snapshot before publishing.
Limits are 128 entries/page, 32 pages, 4,096 entries, and 64 KiB/page. A malformed,
incomplete, reassigned or deleted mapping rejects the refresh and keeps the old
cache. Direct principals remain available during refresh failures.

An unknown alias cannot be admitted. There is no historical replay after a later
refresh; another qualifying Faucet payout supplies another admission opportunity.
Use `preview_subscription(suffix)` on the configuration-verified instance to
check syntax and cached resolution without network calls or state changes. Its
revision and refresh time describe that local cache; preview proves neither
funding nor admission. The browser disables alias-based funding output until the
current preview succeeds.

**Exact funding routes remain distinct.** `E.A.7`, `E.A@A.7`, and a principal
spelling may identify the same effective interest but have different exact
Historian memo routes. Endowment totals are never pooled across spellings. The
original payout suffix, not a rewritten principal declaration, selects evidence.
Only the same `(subscriber, subject, target)` merges thresholds, monotonically
toward less restrictive values. Multiple subscribers watching B.7 keep independent
thresholds, earliest qualifying block indices, and maximum individual amounts.
Changing spelling deliberately changes the funding route.

## Callback and targets

```candid
type PokeTarget = variant {
  subaccount : nat64;
  neuron_nonce : nat64;
};
type PokeMatch = record {
  subject : principal;
  target : PokeTarget;
  first_block_index : nat64;
  max_amount : nat;
};
type PokeNotification = record {
  global_activity : bool;
  matches : vec PokeMatch;
};
service : {
  poke : (PokeNotification) -> ();
}
```

Every receiver implements this single method and required record argument.
`subject` is always explicit, including self matches. The flag is true only
when this subscriber has qualifying global activity in the completed poll;
`matches` independently contains threshold-filtered specific matches, with
either flag value. Global-only calls have a true flag and no matches. Empty
matches alone do not establish global activity. False plus empty is never sent.

This intentionally breaks the previous callback ABI. There are no compatibility
branches, extra arguments, receiver negotiation or fallback methods. Missing
or trapping methods are best-effort failures and do not stop later recipients.

At most one notification call is attempted per subscriber per completed poll.
Specific matches share a combined cap of 256 distinct `(subject, target)` entries.
The first retained identities remain; later activity updates their independent
minimum index and maximum amount. Overflow can omit additional hints, so always
reconcile authoritative history. Ordering preserves subject principal order (byte length, then bytes), then
subaccount numbers, then neuron nonces; neither index nor amount is a sort key.

Within each completed poll, Event Horizon attempts all globally matched
subscribers before specific-only subscribers, in ascending principal order
within each phase. A mixed match receives one poke in the global phase with
both the global flag and the specific matches. This fixed rule is independent of endowment amounts,
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
4. Accept the global flag only when local policy intentionally wants global activity.
5. Discard matches outside the locally approved subject and target set (for self-only consumers, require `subject == self`).
6. Compare `max_amount` with each target's local minimum in that Ledger's atomic units.
7. If no locally allowed global wake or relevant matches remain, return without expensive work.
8. Otherwise coalesce the wake-up and schedule bounded authoritative reconciliation.
9. Make consequential decisions only from verified Ledger/Index data.
10. Keep independent periodic reconciliation through the same per-Ledger path.

Perform authentication and interest/amount checks **before** any asynchronous
Ledger, Index or other expensive inter-canister call. The checks are local;
they require no Event Horizon subscription lookup or external approval call.

```text
poke(notification):
    context = trusted_event_horizon_instances.get(caller)
    if context is absent:
        return

    relevant_matches = filter_by_local_approved_subjects_targets_and_amounts(
        context, notification.matches
    )
    global_wake = context.wants_global_activity and notification.global_activity
    if not global_wake and relevant_matches is empty:
        return

    earliest_hint = none
    if relevant_matches is not empty:
        earliest_hint = minimum(match.first_block_index
                                for match in relevant_matches)
    schedule_coalesced_reconciliation(context.observed_ledger, earliest_hint)

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

## Direct Ledger reconciliation

Each specific match has a required `first_block_index : nat64`: the earliest
qualifying transfer Event Horizon actually observed for that target during the
completed poll, under its effective admission and threshold policy. It is
relative to the authenticated caller's observed Ledger. Global-only `poke([], null)`
has no index; mixed matches retain their specific entries in one global-phase
poke. Zero-argument receivers intentionally discard these hints.

The index and `max_amount` aggregate independently. Transfers to subaccount 7 at
blocks 12,450 (4 ICP), 12,453 (15 ICP), and 12,461 (8 ICP) produce index 12450
and maximum 15 ICP in atomic units. They may describe different transactions.
If Event Horizon's threshold is 1 ICP but your local minimum is 10 ICP, block 100
(2 ICP) followed by block 110 (15 ICP) produces index 100 and maximum 15 ICP.
Inspect the authoritative history with your own policy: the first hinted block
need not itself meet your local minimum.

A direct-Ledger subscriber needs no Index canister. After authenticating the
caller and filtering locally approved interests and amount hints, share a
bounded worker and coalesce accepted hints (including the earliest relevant
pending hint). Use the Ledger context from the trusted caller registry:

- **Canonical ICP:** use the legacy `query_blocks` block-history interface with
  bounded `{ start; length }` ranges. Returned live blocks begin at the response's
  `first_block_index`; use it only for a nonempty block vector. Older requested
  blocks may require the corresponding `archived_blocks` callback and archive
  `get_blocks` retrieval path. Validate returned indices and coverage.
- **Generic ICRC-3:** use `icrc3_get_blocks` with bounded block ranges. Returned
  blocks have explicit IDs; validate those IDs and use the returned
  `archived_blocks` callback arguments to retrieve archived portions when needed.
  An ICRC-3 ID belongs to this observed Ledger, never the ICP admission Ledger.

Subscribers must support the appropriate archive retrieval themselves if they
need that history. Event Horizon never follows archives and serves no historical
transaction data. See the primary [ICP Ledger interface](https://github.com/dfinity/ic/blob/master/rs/ledger_suite/icp/ledger.did)
and [ICRC-3 specification](https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-3).
Preserve `nat64` indices exactly (for example JavaScript `BigInt`, not `Number`).

**Your durable next-unprocessed cursor is authoritative.** If it is 100 and the
hint is 150, do not jump to 150: blocks 100–149 may contain unprocessed activity
from earlier or missed wakes. Reconcile from the existing cursor under your
independently established initialization policy. Prospective-only initialization
is an explicit application choice, not a guarantee from Event Horizon. If the
cursor is already beyond a hint, do not automatically rewind and process history
again. A hint identifies relevant work, accelerates scheduling and can help detect
a lagging Ledger view; it never authorizes skipping history.

Fetching only the hinted block does not establish completion. Continue bounded
pages to discover further relevant transactions, with persistent cursors,
idempotent processing and sensible per-run work limits. Authenticate and filter
before scheduling or reading; an unsolicited or weakly thresholded subscription
must not turn a supplied index into an unbounded scan. Coalesce multiple accepted
pokes into the same worker, prevent overlap, and retain independent periodic
reconciliation. A poke is not a receipt: independently verify authoritative
Ledger records before consequential financial decisions. Direct reads can fail,
be temporarily stale or need archives; retain pending work and bounded retries.
An immediate query is not guaranteed sufficient.

The following Index catch-up flow is a separate valid approach for applications
whose reconciliation depends on an Index. Direct-Ledger readers need no
Index-specific five-second wait. An Index-dependent application may optionally
read the Ledger using these hints when its Index is behind; this hybrid policy
belongs to the application and adds no Event Horizon configuration setting.

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

Another party can endow the same subscriber/subject/target declaration with a less restrictive threshold. Event Horizon permanently keeps the least restrictive effective threshold for that subscribing relation. Other watchers retain their own thresholds. For example:

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

Similarly, `poke([], null)` contains no specific target match. An empty first vector with nonempty external hints still requires subject filtering. A subscriber uninterested in global activity can immediately return. Event Horizon provides no retries or acknowledgements, so independent periodic reconciliation remains required.


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
retain at most 256 subject-qualified specific targets per subscriber per poll, so this local scan
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

## General wake-up consumers

A consumer intentionally interested in global activity still implements
`poke(PokeNotification)`. Authenticate the caller, check `global_activity`
against intentional local global interest, and coalesce bounded work. Empty
matches do not imply a global match. Existing Index canisters are not assumed
to export this method or to complete reconciliation before other subscribers.

## Jupiter receiver deployment dependency

Jupiter Faucet and Disburser's current receiving methods are incompatible with
this callback. Their separate repository must implement the exact Candid above
and coordinate its receiver rollout with Event Horizon before relying on
notifications. Authenticate the trusted Event Horizon principal, select
`match.subject == self`, and preserve existing local targets and thresholds
before scheduling authoritative work. No Event Horizon fallback will bridge
this change. Disburser-to-Historian forwarding is a separate contract and is
not assumed to change. This release does not modify or deploy Jupiter.
