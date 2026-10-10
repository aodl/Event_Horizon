# Funding, economics, and pricing

This is the authoritative explanatory reference for Event Horizon funding and
admission pricing. Normative protocol rules remain in [`SPEC.md`](../../SPEC.md).

## Productive endowments

A subscription is not a one-time payment into Event Horizon. The reviewed
Jupiter workflow is:

1. A patron transfers ICP to Jupiter Faucet's configured NNS neuron staking
   account with a Jupiter-recognised declaration memo.
2. A qualifying transfer contributes to productive neuron stake.
3. The neuron earns variable voting maturity.
4. Jupiter Disburser routes the age-neutral base component of maturity-derived
   ICP to Jupiter Faucet.
5. Faucet allocates proportional payouts to eligible endowment routes.
6. An Event Horizon route receives raw ICP with its exact declaration suffix
   preserved.
7. Event Horizon requires both that authorised payout and complete Historian
   evidence before permanently admitting the declaration.
8. Event Horizon converts retained incoming raw ICP into cycles for Ledger
   polling, subscriber pokes, memory, execution, and inter-canister calls.

Endowed principal never enters Event Horizon's liquid operating balance. The
recurring maturity-derived payouts attributable to productive principal help
fund the service. Reward rate, payout amount, payout cadence, conversion
outcome, and uninterrupted service are not guaranteed.

The displayed account, range, or global requirement is the minimum qualifying
cumulative endowment for one exact declaration route. The original staking
transfer is not an admission event. The reviewed production Faucet uses a
604,800-second (seven-day) stake-recognition delay, and later payout processing
can add further delay. Repeated endowments to the identical route contribute
cumulatively, and Faucet continues considering eligible endowment records in
later payout rounds. Any patron may endow a declaration; control of the
subscriber canister is not required.

Cumulative endowment principal and cumulative rewards distributed are separate
accounting concepts. Once admitted, a subscription is permanent. Further
endowment creates no priority, faster polling, subscriber balance, or private
cycle allocation.

## Dynamic admission prices

The sole oracle is the CMC `get_icp_xdr_conversion_rate` integer. Event Horizon
attempts roughly one observation per UTC day, stores at most one success for
that date, and neither retries within the day nor backfills missed dates. The
rolling window is the current UTC day and previous 1,460 day buckets, for at
most 1,461 observations.

The first successful observation establishes both launch floor and latest rate,
producing 10 ICP account, 20 ICP range, and 1,000 ICP global requirements. With
retained minimum `F` and latest eligible rate `C`:

```text
account = ceil(10 × F / C)
range   = ceil(20 × F / C)
global  = ceil(1000 × F / C)
```

All calculations use checked integer arithmetic and round each class upward to
whole ICP independently. A rounded range price need not be twice the rounded
account price, and a rounded global price need not be exactly 100 times it. The
stable `Price` stores account and global values. Range remains exactly derived
from its independent 20 ICP basis using:

```text
ceil(20F/C) = ceil(ceil(1000F/C) / 50)
```

Range width does not affect price. Prices freeze seven 24-hour days before the
next month's UTC boundary. Observations at or after the freeze wait for a later
epoch; stale eligible data carries current prices forward. Admission compares
the exact Historian route total with the current requirement at evaluation
time.

## Pricing-policy state boundary

The prior deployment could contain a stable global price calculated from the
old 100 ICP basis. An ordinary upgrade would preserve that value while the new
code derives range by 50; for example, stored global `100` would display range
`2`, not `20`. Carrying old frozen/current prices forward would also postpone
the new global policy.

No general migration machinery was added. For this exceptional prelaunch
change, the selected strategy is a controlled reinstall only after an operator
verifies that production has no subscriptions, pending financial actions, or
other obligations requiring preservation and separately approves the reset.
Reinstall destroys pricing observations and epochs, cursors, subscriptions,
profile/configuration state, and pending funding state. A fresh reader is
prospective and will not replay historical Faucet payouts. It must bootstrap
before new endowments are accepted. Subsequent ordinary releases remain
argument-free, state-preserving upgrades.

## Protocol kickstart and subscriber backing

Three funding categories must remain distinct:

- **Protocol endowment:** a planned 3,000 ICP productive endowment through
  Jupiter Faucet, intended to generate recurring baseline payouts. It is not a
  confirmed deposit, guaranteed yield, hardcoded constructor value, or liquid
  cycle balance.
- **Subscriber endowments:** qualifying account, range, and global routes that
  add productive backing and enable permanent subscriptions.
- **Liquid cycles bootstrap:** cycles immediately available while initial
  Jupiter payouts are pending. Reaching and sustaining Continuous-mode entry
  may require at least 100 T liquid cycles; its exit threshold remains 60 T.

At reviewed Jupiter source revision
`a484ae10eaee9b47b64657d1f7e5d70eec8e4fe7`, the source-controlled `X` alias
accepts any nonempty suffix within the complete 32-byte ASCII memo limit, routes
raw ICP to canonical Event Horizon, and preserves the suffix exactly.
`X.protocol-seed` is 15 bytes, therefore routes payout memo `protocol-seed`.
Event Horizon's parser rejects that suffix as a subscription declaration, as a
repository unit test confirms, so it does not create an unintended subscriber.

This establishes source-level compatibility, not a reservation or mainnet
publication. Before any transfer, operators must confirm that the deployed
Jupiter policy/configuration still matches the reviewed source and explicitly
approve the candidate memo. No special Event Horizon API is required.

## Pooled sustainability

All qualifying Faucet payouts and other eligible incoming raw ICP share one
operating reserve. A 1,000 ICP global reference endowment is not a prepaid
subscriber-specific cycle allocation. The result depends on NNS rewards,
Jupiter allocation, total productive stake, ICP/XDR rates, actual polling and
poke rates, subscriber fan-out, memory and execution, inter-canister calls,
fees, conversion results, receipt timing, liquid cycles, and any enabled
surplus diversion. Global service can cost considerably more than a specific
account subscription.

The following figures are hypothetical illustrations, not yield promises,
forecasts, or mainnet burn measurements. Assuming 7% gross annual reward APY, a
25% maximum age multiplier, a 5.6% illustrative effective base payout rate,
and the listed ICP prices:

| Endowment | Illustrative annual ICP payout | At $2.50 | At $3.50 |
|---|---:|---:|---:|
| Protocol: 3,000 ICP | 168 ICP | $420 | $588 |
| Each global: 1,000 ICP | 56 ICP | $140 | $196 |

Synthetic PocketIC fan-out measurements imply about 5.49 million marginal
cycles per global poke; one poke per second would be about 173 T cycles/year. A
hypothetical one-second poll rate at 10 million cycles per poll adds about 315 T
cycles/year in shared polling expense. These synthetic assumptions do not prove
that 1,000 ICP is sufficient or insufficient for any production workload.

## Adaptive surplus diversion

Event Horizon keeps all funding initially. Each seven-day period whose observed
liquid-cycles minimum remains at least 150 T raises the fraction of future raw
ICP eligible for diversion by five percentage points, up to 95%. A 100–150 T
period lowers it by five points; below 100 T resets it to zero. No surplus leaves
while current liquid cycles are below 150 T.

Retained funding is converted and confirmed before a corresponding surplus leg
can move. The optional receiver is immutable install-time configuration.
`None` disables diversion. Subscriber priority never depends on funding source,
and no subscriber accounting, forecast, alternate oracle, or burn-based dynamic
pricing mechanism exists.
