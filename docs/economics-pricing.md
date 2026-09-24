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

## Adaptive surplus diversion

Event Horizon keeps all funding initially. Each seven-day period whose observed liquid-cycles minimum remains at least 150 T raises the fraction of future raw ICP eligible for diversion by 5 percentage points, up to 95%. A moderately weak period from 100 T inclusive to 150 T exclusive reduces that fraction by 5 points, while a period falling below 100 T resets it to zero. The minimum is the lowest sample taken by the existing hourly funding lane, not a continuously measured theoretical minimum. A long inactive interval never manufactures multiple healthy epochs.

No surplus leaves while the current liquid balance is below 150 T. For an active percentage `P`, two transfer fees are removed before `floor((B - 2F) * P / 100)` is assigned to surplus; all rounding remainder stays retained. Event Horizon's retained share is converted to cycles before the corresponding surplus share can leave. If minting fails, refunds, remains Processing, or ends terminally, no associated surplus moves. A second 150 T check after mint can cancel the surplus leg and leave that ICP for a later Event Horizon funding plan.

The receiver is immutable once production is finalized, and Event Horizon neither calls nor controls it. The production constant is currently `None`, which disables the controller and resets its level to zero. Direct donations may become part of this same common funding/surplus flow. Subscriber service priority is never determined by who supplied the ICP; no subscriber-specific accounting, forecast, oracle, moving average, or burn model exists.
