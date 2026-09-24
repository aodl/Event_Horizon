# Economics and pricing

All endowments fund one common Event Horizon cycles pool. Different admission tiers create no allowance or priority.

The sole oracle is the CMC `get_icp_xdr_conversion_rate` integer. Event Horizon attempts roughly one observation per UTC day, stores at most one success for that date, and neither retries within the day nor backfills missed dates. The rolling window is the current UTC day and previous 1,460 day buckets. At most 1,461 small observation records are retained, about 187 KiB at the stable encoding's conservative 128-byte bound plus map overhead.

The first success establishes both launch floor and latest rate, producing 10 ICP account, 20 ICP range, and 100 ICP global prices. Thereafter, with retained minimum `F` and latest eligible `C`:

```text
account = ceil(10 × F / C)
range   = ceil(20 × F / C)
global  = ceil(100 × F / C)
```

All calculations use checked integer arithmetic. Range pricing is independently equivalent to the 20-ICP basis, not twice the rounded account price. The implementation uses the identity `ceil(20F/C) = ceil(ceil(100F/C)/5)` so no stable pricing migration is needed. Prices freeze seven 24-hour days before the next month's UTC boundary. Observations recorded at or after the exact freeze wait for a later epoch. If the latest eligible CMC timestamp is more than seven days old, the current prices carry forward.

Event Horizon does not price ranges according to the number of subaccounts they contain. A single subaccount can generate more activity than a large range, and Event Horizon already reads every ICP Ledger transaction. Range subscriptions therefore use a simple 20-ICP reference basis—twice the standard account reference basis—while global Ledger subscriptions use a 100-ICP basis.

Admission compares the exact Historian route total with the current requirement at evaluation time. Existing admissions remain permanent. Larger endowments buy no additional service.

The backend records the last successful pricing call's `Call::get_cost()` in stable pricing state for test and operational analysis; it is deliberately absent from the public query. The final validation report records any measurable value available in the executed environment.

The daily observation is nonessential. Event Horizon obtains the pending call's current `get_cost()` and issues it only when `liquid cycles >= RESERVE_PROTECTION_CYCLES + call cost`. A reserve-protected skip consumes the UTC day's single opportunity and leaves all pricing state unchanged, allowing the existing stale-data carry-forward rule to apply. Ledger polling, CMC funding recovery, and existing admissions continue independently.
