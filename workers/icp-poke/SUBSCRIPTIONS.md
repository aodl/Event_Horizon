# Worker permission versus an Event Horizon subscription

## The workers do not buy a subscription

Each off-chain worker needs its own signing identity and checkpoint. The
canonical backend's `allowed_poke_callers` authorises that public principal
to send a wake. The worker needs no paid Event Horizon subscription,
productive endowment, API account or ICP wallet balance.

Use a receiver canister principal when composing a customer subscription
memo. The off-chain identities are only for signing worker requests.

## What a customer subscription means

An actual Event Horizon subscription means: notify a particular **subscriber
canister** when matching activity occurs for a chosen account or target. The
subscriber receives the callback; the subject owns the account being watched.
They can be the same canister.

The worker discovers candidate interests from authentic Faucet payouts to Event
Horizon. It does not receive a webhook from the subscription website and does not
add an interest merely because someone sent ICP into the initial staking flow.
When it sees a payout, it wakes the backend and consults existing preview,
pricing and Historian queries to learn eligible candidate targets. It checks
subsequent ledger activity against these candidates. The existing queries
do not expose an authoritative list of admitted subscriptions. The worker
can send extra wakes or learn interests later than the backend.

Event Horizon independently checks the same unread history, admits actual
subscriptions, matches transactions, and sends its usual notification. The
subscriber then reconciles its own ledger or index before applying business
logic. The zero-argument worker poke carries no transaction, recipient or worker
cursor. The backend retains fallback polling and reserve controls and still
processes its own unread ledger history.

## First real subscription: a canister's default ICP account

You must first identify a deployed receiver canister that implements the callback
and has its own reconciliation logic. A random principal or ordinary wallet will
not become a receiver simply because its address is in a memo.

1. Configure the receiver to trust canonical Event Horizon
   `eo6ei-gaaaa-aaaar-qchra-cai` as its notification caller. It does not need to
   trust any worker principal. Normal application receivers use the documented
   `poke(PokeNotification)` interface and filter locally approved interests.
   The worker-to-backend endpoint remains `poke()`.
2. Open [Event Horizon](https://www.event-horizon-icp.com/).
3. In **01 / Select**, choose the canonical ICP instance and wait for its
   configuration and current price to load.
4. In **02 / Compose**, enter the receiver's canister principal, select
   **Same canister**, choose **Single subaccount**, and enter `0`. Leave the
   minimum amount blank for every incoming transfer, or set the intended
   transfer threshold. This threshold is separate from the admission endowment.
5. In **03 / Complete**, copy the generated exact declaration. Conceptually
   it is `E.<subscriber>.0`; let the builder handle compact principals and the
   32-byte memo limit. Do not substitute hypothetical aliases from examples.
6. Follow the [Jupiter Faucet](https://www.jupiter-faucet.com/) productive
   endowment instructions with that exact complete memo and the current required
   amount. Use the verified Faucet staking flow. Sending ICP directly to Event
   Horizon's default account does not establish a subscription.
7. Wait for a genuine Faucet payout carrying the declaration and complete
   Historian evidence of sufficient cumulative endowment for that exact route.
   The project's reviewed Faucet configuration specifies a seven-day stake
   recognition delay, with further payout delay possible. One-second polling
   cannot remove those delays. Admission is permanent once established.

The exact amount comes from the verified frontend/current `get_pricing`, not a
fixed number copied from a previous conversation. Amounts and spelling matter:
semantically equivalent declarations can be different exact funding routes and
their endowments are not pooled. Check the amount and generated memo before any
payment. Endowment principal follows Jupiter's productive staking path rather
than becoming Event Horizon's liquid balance; this is not a monthly invoice.

Before the first funded subscription, the project's deployment runbook requires
verification of the live Faucet E routing, Historian alias and exact-route
support, and the selected receiver's deployed callback. Deploying Event Horizon
alone does not prove those external integrations are active. These checks do not
prevent installing the workers or testing their allowlisted signed pokes.

## A concrete lifecycle

1. A patron endows the route for receiver C's default ICP account.
2. Later, Faucet pays Event Horizon with C's declaration in its preserved memo.
3. Independent workers observe that payout and ask Event Horizon to scan early.
4. Event Horizon independently validates and admits the subscription; the
   workers learn a candidate if their query evidence is eligible.
5. A later incoming transfer to C's default account meets the threshold.
6. Each worker can send a zero-argument wake. Event Horizon scans its own unread
   ledger history and notifies C once per completed poll according to its rules.
7. C reconciles its own account history and acts on verified information.

Independent on-chain polling continues if both workers are down or have not yet
learned a candidate. Worker acknowledgment means the wake was accepted, not that
C was notified or finished processing. Subscriber delivery remains best effort.

## Separate generic-backend dogfooding

A generic Event Horizon instance may itself subscribe to canonical ICP Event
Horizon for transfers to its default ICP account. That requires its immutable
`icp_admission_notifier` plus a separately funded `E.<generic-instance>.0` route.
It is a different optional setup and is not required for these off-chain workers.

Sources: [Subscriber guide](https://github.com/aodl/Event_Horizon/blob/5f7ee49d22b3144b490497a97ee03d1a064c55cd/docs/subscriber-guide.md),
[backend specification](https://github.com/aodl/Event_Horizon/blob/5f7ee49d22b3144b490497a97ee03d1a064c55cd/SPEC.md),
[activation runbook](https://github.com/aodl/Event_Horizon/blob/5f7ee49d22b3144b490497a97ee03d1a064c55cd/docs/operations/deployment.md#d-activate-subscriptions-separately--deferred-gate),
and [generic-instance setup](https://github.com/aodl/Event_Horizon/blob/5f7ee49d22b3144b490497a97ee03d1a064c55cd/docs/operations/deploying-an-instance.md#optional-icp-admission-notifications).
