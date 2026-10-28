# Ordinary-polling projections

Four corrected inputs each produced 63 simulations in the existing cadence model,
using seed 20261028 and the unchanged current policy. Raw inputs, configurations,
outputs and digest receipts are retained. Workload IDs `expected` and
`expected_generic` mean **idle** ICP and generic observation here; they do not mean
the historical example workload. The initial adapter's invalid workload IDs were
rejected before simulation; no result from that attempt is accepted.

Generate inputs with `tools/prepare-cycle-optimisation-model.py` from the tracked
fixed-work CSV/receipt, then run:

```bash
./tools/scripts/calibrate-cadence model --input DIR/measurements.json --config DIR/model-config.json --output DIR --seed 20261028
```

The initial execution used the already built native model binary (digest in
`receipts.json`), without another build or concurrent PocketIC measurement. The
supported wrapper runs the same unchanged model implementation. Both use actual
median rows from three complete samples. Maintenance coefficients are unavailable;
the model's zero placeholder is **not** evidence of free maintenance. These outputs
also omit storage billing and cannot estimate total expenditure or mainnet runway.
Actual full-day scheduler measurements are separate.

`summary.csv` selects no-incoming-funding equal-balance and latency sensitivity
results. Equal-balance rows are ICP, at 6T/110T initial cycles over 90 days. Both
architectures have fixed-tier economics and 30/90/365-day workload rows in the raw
JSON. Traffic and productive backing are zero, so even synthetic funding profiles
produce no incoming cycles. Continuous completion intervals 1/2/5/10 seconds are
explicit assumptions. Poll counts are exact integer quotients of polling cost and
the calibrated idle cost, with zero remainder.

At 110T and one-second completion, the modeled first downgrade with zero globals
moves from 3,780,616 to 3,883,961 seconds; with 1,024 idle globals, from 2,565,478 to
3,884,057 seconds. This means more polls in Continuous before downgrade. It is not
a claim that daily savings equal fixed-work savings. All projected backlogs remain
zero because the declared workloads are idle; actual traffic/lag evidence is in
the independent scheduler CSV.
