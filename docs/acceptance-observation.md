# Acceptance cycle observation

`./tools/scripts/mainnet-observe [backend-id] [frontend-id]` is a read-only operator command. Defaults are the canonical ICP backend `eo6ei-gaaaa-aaaar-qchra-cai` and frontend `ej7c4-lyaaa-aaaar-qchrq-cai`. It writes a UTC-stamped ignored `observations/` directory containing revision, status, backend instance/pricing/logs, and management-canister metrics.

Run hourly or daily during focused acceptance testing and at least immediately before and after a weekly Faucet top-up. `canister_metrics` requires controller authority; after controllers are removed the script records that metrics are unavailable without failing the snapshot.

For two timestamps, subtract each cumulative `cycles_consumed` counter. Divide the total delta by elapsed days for cycles/day and divide each category delta by the total for category contribution. The counters distinguish memory, compute allocation, ingress induction, instructions, request/response transmission, HTTP outcalls, and explicitly burned cycles, so incoming top-ups do not obscure burn. Also record polling tier, liquid cycles, cursor movement, and subscriber counts from status and HEALTH logs. Current cadence thresholds are not economically proven; this controlled period exists to calibrate them.
