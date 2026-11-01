# Acceptance cycle observation

`./tools/scripts/mainnet-observe [backend-id] [frontend-id]` is a read-only operator command. Defaults are the canonical ICP backend `eo6ei-gaaaa-aaaar-qchra-cai` and frontend `ej7c4-lyaaa-aaaar-qchrq-cai`. It writes a unique UTC-stamped ignored `observations/` directory containing revision, commands, per-read exit results, public status, backend instance/pricing/logs, and attempted management-canister metrics. Prepare the locked SDK dependency with `cargo run -p xtask -- frontend_setup` on a fresh checkout.

Begin with an unsubscribed observation period after the separately authorised
backend/frontend upgrades. Ledger progress, cycles, pricing, funding/recovery
maintenance and frontend checks do not require Jupiter upgrades or live callback
evidence. No pokes are expected without an admitted subscriber. A missing alias
query may produce `ALIASES_UNAVAILABLE`: record it as an external subscription
readiness limitation, and investigate if polling stalls or other problems occur.
Account explicitly for documented archive-skipping `HISTORY_GAP` records; they
do not prove complete historical processing. Unexpected reader errors, failed
cursor progress and inadequate cadence require separate assessment. Use timestamped
reader evidence; the frontend's Ledger history total is not a processing counter.

Run hourly or daily during focused acceptance testing and, after separately
authorised activation, around each observed Faucet funding receipt. All script
reads use an anonymous identity. Native
management status/metrics queries explicitly route through the observed
canister using the pinned SDK; CLI 1.2.0 cannot supply this routing override.
`canister_metrics` requires controller authority and therefore normally rejects
these anonymous reads, even while the canister has controllers. The script
records every failed command and returns nonzero while retaining all successful
partial observations. A failure never means empty logs or zero consumption.
Verify `results.tsv` before drawing conclusions.

For two independently obtained successful, controller-authorized metrics readings, subtract each cumulative `cycles_consumed` counter. Divide the total delta by elapsed days for cycles/day and divide each category delta by the total for category contribution. The counters distinguish memory, compute allocation, ingress induction, instructions, request/response transmission, HTTP outcalls, and explicitly burned cycles, so incoming top-ups do not obscure burn.

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

The receipt/fan-out checks apply when that activity exists; their absence during
unsubscribed observation is not a callback failure. Before accepting funded
subscriptions, satisfy the [activation gate](deployment.md#d-activate-subscriptions-separately--deferred-gate)
and decide whether measured cadence and cycle reserves provide the intended
notification service level. Any live end-to-end test requires separate operator
authorisation. Controller removal retains [its stricter checklist](controller-removal.md).

Compare observed long-term receipts with observed expenditure over equivalent
windows. Distinguish startup funding coverage, minimum liquid cycles and mature recurring economics; between-poll waiting, persistent reader lag and reserve suspension also have different causes. See the [calibration validation](../codex/cadence-calibration-validation.md). A planned 3,000 ICP protocol endowment and 1,000 ICP global reference
prices are not evidence of sufficient yield by themselves. Synthetic fan-out
or one-second polling figures are planning illustrations, not mainnet burn.
Current cadence thresholds are not economically proven; this controlled period
exists to calibrate them using the existing read-only observation tooling. Use
the [local cadence-calibration guide](../development/cadence-calibration.md) to
compare these observations with PocketIC measurements and offline projections;
do not recreate the synthetic workload by spending real ICP.
