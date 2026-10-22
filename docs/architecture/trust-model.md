# Trust model

For a finalized instance, verify the canister principal, empty controller list, public module hash, reproduced generic backend hash, `get_instance` values for the observed Ledger, SNS Root, and surplus recipient, recent public CONFIG/HEALTH logs, the Ledger principal independently, the Jupiter alias mapping, and the fixed protocol anchors in source/module. The complete tuple—not module hash alone—identifies the instance.

Wasm-level fixed protocol anchors are the ICP Ledger, CMC, NNS Governance, Jupiter Faucet, and Jupiter Historian. Instance-level immutable configuration is `observed_ledger`, optional `sns_root`, and optional `surplus_canister`. The module hash proves common implementation and policy. `get_instance` reports the canister's immutable configuration and the frontend compares that report with its registry; it does not independently prove which module is installed or whether controllers remain. Module hash and controller state are separate operator/reviewer checks. The initial CONFIG log supplies additional public evidence of the installed tuple.

Event Horizon is a latency aid. Subscriber reconciliation is authoritative.
Any patron can finance a qualifying account, range, neuron or global declaration
without the subscriber's consent, including unwanted targets or weaker effective
thresholds. An authenticated Event Horizon callback proves its source, not that
the subscriber wanted the subscription. Local interest/authorisation is a
subscriber boundary, separate from Event Horizon's funding-based admission.

- Canonical ICP defines Faucet admission for every instance. It also defines observed activity and `chain_length` boundaries only for the ICP instance; non-ICP observed activity and `log_length` boundaries come from the configured ICRC-3 Ledger.
- Jupiter Faucet origin plus a complete Jupiter Historian exact-route
  productive-endowment total define admission. Jupiter Faucet's source policy
  owns alias mappings; Historian publishes permanent identity aliases and verifies history and exact route totals; it does not
  define those mappings.
- The CMC's ICP/XDR conversion-rate query is the only pricing oracle.
- Liquid cycles observed hourly are the only adaptive-surplus health signal.
- `get_instance`, `get_pricing` and local-state-only `preview_subscription` are the backend's only production application queries. Preview resolves a proposed declaration, without admitting or exposing the subscription registry.
- Range declarations are protocol-bounded to 256 admission-time watched-account merges; they add no per-transfer scan, per-range scheduler, or additional poke allowance.
- NNS Governance is a fixed ICP trust anchor. SNS neuron support additionally trusts immutable `sns_root`, its bounded Root response, and the observed Ledger's Governance-default minting account; Root and Governance are not polling dependencies after profile verification.
- Poke targets, `first_block_index` and `max_amount` are non-authoritative hints. The minimum qualifying index and maximum individual amount may refer to different transactions; neither authorizes skipping the subscriber’s durable cursor. Scoped receivers should normally authenticate the caller, resolve its immutable Ledger context, then reject unapproved targets and below-local-threshold amounts locally before expensive Ledger/Index calls. Their approved-interest registry is independent of admitted subscriptions.
- Global interest must be intentional local policy: scoped consumers filter subject-qualified matches; global consumers additionally accept the explicit `global_activity` flag. Specific matches and global activity are independent; see the [subscriber guide](../subscriber-guide.md).
- For a subscriber trusting multiple Event Horizon instances, the authenticated caller defines the observed-ledger context of every poke.
- Public native status and logs remain the operational interface.
- The mutable frontend presents pricing obtained by a direct read-only backend query but cannot change backend admission decisions. Its embedded assets and response headers are certified. HTML is non-stale; mutable unversioned JavaScript, CSS, and SVG require revalidation. A one-time `?v=2` script/style transition bypasses previously cached immutable URLs, and stale asynchronous backend completions are ignored. Backend admission remains authoritative.

Daily samples can be missed, and the observed rolling minimum is only the lowest successful daily observation recorded by Event Horizon. It is not a market low. Stale data carries prices forward. Pokes can fail, be delayed, or arrive before Index visibility.

Productive endowment principal remains in Jupiter's staking path. Event Horizon
trusts authorised raw-ICP Faucet payouts and Historian evidence, then converts
retained incoming ICP through the CMC. It does not guarantee NNS yield, Jupiter
payout size or cadence, ICP/XDR conversion outcome, or uninterrupted service.
The planned 3,000 ICP protocol endowment is an intended external funding
arrangement, not immutable instance configuration or evidence of an existing
deposit.

Surplus epochs likewise use an observed hourly minimum, not continuous monitoring or a burn forecast. The current-balance gate and retained-first ordering protect service even when the stored level is high. The immutable receiver is trusted only as the destination account owner: Event Horizon calls no receiver endpoint, grants it no control, and makes no claim about how it governs or spends received ICP. Direct donations and Faucet funding share the same pool; funding source never changes subscriber priority.

Controller removal remains an irreversible action after a controlled observation period. Before it, the installed surplus recipient must be deliberately disabled or equal the final reviewed receiver. The same module supports either choice; there is no recipient environment variable, build argument, feature, source substitution, or recipient-specific Docker build. The backend contains no administrative, destination, withdrawal, recovery, install-code, or self-upgrade method.

Historian identity aliases identify public canisters; publication grants neither
control nor consent. The backend trusts complete digest-verified additive
snapshots from its fixed Historian principal, retains the previous cache on
failure, and resolves aliases only at admission. Browser preview consults this
cache on the verified backend. Exact funding-route bytes stay unchanged.
All hints require locally approved `(subject, target)` policy. Explicit subjects
and an independent global flag prevent external-only matches from masquerading
as global activity. Jupiter receivers must adopt this breaking ABI separately.
