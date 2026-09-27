# Subscriber guide

Implement and authenticate the target-aware callback:

```candid
type PokeTarget = variant { subaccount : nat64; neuron_nonce : nat64 };
type PokeMatch = record { target : PokeTarget; max_amount : nat };
service : { poke : (vec PokeMatch) -> (); }
```

Jupiter suffixes accept `<subscriber>`, `<subscriber>.<number>[:<amount>]`, `<subscriber>.<start>-<end>[:<amount>]`, and corresponding neuron forms using `n<nonce>` or `n<start>-<end>`. Numbers are canonical decimal `u64`. An inclusive range contains 2 through 256 targets and may begin anywhere. The complete routed memo still must fit 32 bytes.

Numeric subaccount `N` means subscriber ownership with `24 zero bytes || N.to_be_bytes()`. Neuron nonce `N` derives the staking subaccount with controller equal to the subscriber principal. ICP uses NNS Governance as owner; an SNS-token instance uses its verified SNS Governance. This excludes hotkeys, permission holders that are not the derivation controller, arbitrary governance owners, and arbitrary accounts.

A safe handler follows this order:

1. Authenticate `caller` as a trusted Event Horizon instance.
2. Return cheaply on `poke([])` if global activity is irrelevant; otherwise reconcile the global cursor.
3. Ignore unknown targets.
4. Compare `max_amount` with the application's own local threshold and return if none qualify.
5. Only then reconcile authoritative Ledger or optional Index state.
6. Perform consequential work solely from reconciled state.

`max_amount` is the largest individual qualifying incoming transfer Event Horizon observed for that target during the completed poll, in raw observed-token atomic units. It is a prefilter hint, not proof of payment. For example, an ICP policy requiring `1_000_000` e8s returns on a `300_000` hint and reconciles on `2_000_000`.

Specific matches take precedence over simultaneous global activity. `poke([])` means global-only activity. Non-empty targets are ordered as numeric subaccounts ascending and then neuron nonces ascending, with at most 256 retained targets per subscriber per poll. Any omission affects wake-up latency only. Event Horizon provides no retries or acknowledgements; independent periodic reconciliation remains required.
