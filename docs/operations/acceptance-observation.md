# Acceptance cycle observation

`./tools/scripts/mainnet-observe [backend-id] [frontend-id]` is a read-only operator command. Defaults are the canonical ICP backend `eo6ei-gaaaa-aaaar-qchra-cai` and frontend `ej7c4-lyaaa-aaaar-qchrq-cai`. It writes a UTC-stamped ignored `observations/` directory containing revision, status, backend instance/pricing/logs, and management-canister metrics.

Run hourly or daily during focused acceptance testing and around each observed
Faucet funding receipt. `canister_metrics` requires controller authority; after
controllers are removed the script records that metrics are unavailable without
failing the snapshot.

For two timestamps, subtract each cumulative `cycles_consumed` counter. Divide the total delta by elapsed days for cycles/day and divide each category delta by the total for category contribution. The counters distinguish memory, compute allocation, ingress induction, instructions, request/response transmission, HTTP outcalls, and explicitly burned cycles, so incoming top-ups do not obscure burn.

Correlate, without treating any one item as sufficient proof:

- recurring raw-ICP Faucet receipts on the Protocol ICP Ledger;
- retained ICP transfers and successful CMC conversions;
- liquid-cycle changes and cumulative cycles consumed;
- the inferred cadence tier and actual poll rate;
- actual global callback attempt rate, subscriber fan-out, and measured
  incremental notification cost;
- baseline burn during intervals without global notification work;
- fees, refunds, conversion outcomes, and any enabled surplus diversion;
- productive-endowment evidence separately from rewards already distributed.

Compare observed long-term receipts with observed expenditure over equivalent
windows. A planned 3,000 ICP protocol endowment and 1,000 ICP global reference
prices are not evidence of sufficient yield by themselves. Synthetic fan-out
or one-second polling figures are planning illustrations, not mainnet burn.
Current cadence thresholds are not economically proven; this controlled period
exists to calibrate them using the existing read-only observation tooling. Use
the [local cadence-calibration guide](../development/cadence-calibration.md) to
compare these observations with PocketIC measurements and offline projections;
do not recreate the synthetic workload by spending real ICP.
