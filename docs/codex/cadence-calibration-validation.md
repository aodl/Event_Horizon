# Cadence calibration validation

## Scope and provenance

This validation supersedes the calibration model conclusions previously
generated from the same raw PocketIC dataset. Review found that the first
model coupled global notifications to specific matches, gave candidates
different starting resources without a separate equal-balance comparison,
and used one assumed Continuous completion rate. Those conclusions, including
the former 125 T Continuous-entry recommendation, are withdrawn.

Production thresholds, polling delays, funding behaviour, notification
semantics, stable memory, Candid, and immutable configuration remain
unchanged. No mainnet call, deployment, reinstall, upgrade, top-up, ICP/cycles
transfer, or controller operation was performed.

The final review output is locally available at
`observations/cadence/final-20261012/` and is intentionally ignored.
Its `measurements.json` and `measurements.csv` are byte-identical copies of the
previously completed full measurement dataset:

- JSON SHA-256
  `d7ec1ed90b8390188df62dd8cab882705ad2353e1b5e8ec006aba5c12f505797`;
- CSV SHA-256
  `92942cc93b53f1a54355678857c8e4ff25082abd25412d6cea98604df0620eb9`;
- source revision `41e088f471e807e202f54b53009d1d56e37910ba`, with review changes recorded
  as a dirty working tree;
- PocketIC 15.0.0, one application subnet, normal cost schedule, and seed
  `20261011`;
- debug/test backend Wasm SHA-256
  `6f20cf491802d35555d4a76cffddc31d56147232f4cb257d7e5ae947c3be3572`;
- 54 complete raw scenarios and 50 threshold observations, with no cap,
  truncation, or omitted measurement failure; and
- 205 final deterministic simulations from the complete dataset.

Final ignored output hashes are `8f72f435b5da84407ab35cf65fa1a634e26d72f55ce4f6696c8f3b8223d527ff`
for `simulations.json`, `3b8facbf4f0f07d63588f8602b18f74e8d99e46d849a7a80ae042facf6694010`
for `threshold-comparison.csv`, and `2a5ee58fd9b28babb742de49aa96824f0dff0b283afe2c159fa07e3af64d0c64`
for `report.md`. The copied final `measurements.json` still has the raw hash above.

Rerunning the full PocketIC matrix was unnecessary because the corrections
required no new empirical coefficient. A fresh final smoke run completed nine
scenarios and all 50 threshold observations at
`observations/cadence/smoke-final-20261012/`.

## Corrected notification model

Global and specific notification counts are now independent:

- every nonempty completed poll notifies each global subscriber once,
  regardless of whether a block matches a specific target;
- an empty poll sends no global notification;
- specific notifications use explicit matching-block and distinct-subscriber
  distribution assumptions and remain capped once per subscriber per poll;
- a subscriber in both registries is charged once when a specific payload
  takes precedence over the global empty hint;
- redundant matches in one poll are coalesced; and
- recipient execution remains outside Event Horizon's burn.

The final batch correction counts polls that actually process live activity,
including the sequence of backlog draining and subsequent empty polls. A
1,000-block backlog that fits in one poll sends one global poke across 100
polls, rather than the former aggregate estimate of 100. Tests compare the
cycle-skipping batch projector with an independent bounded per-poll reference,
including capacity-limited bursts and sustained traffic. The default 4,096
blocks-per-poll capacity is an explicit modelling assumption; completing a
4,096-block PocketIC case does not prove a hard protocol ceiling. Focused
tests also cover one block per poll, irrelevant-only activity, empty polls,
ten blocks coalesced into one global poke, disjoint specific targets, and mixed
global/specific delivery. The expected workload assumes one block per second,
5% specifically matching blocks, one distinct specific subscriber per
matching block, one global subscriber, and five specific subscribers. It does
not assume that every matching transaction wakes all five subscribers.

The empirical notification coefficient remains 5,488,565 backend cycles per
additional dispatch. The underlying 0/1/10/100/256/1,024-subscriber
measurements were not changed.

## Funding consistency

The expected workload has productive backing for one global declaration,
three account declarations, and two range declarations. The global reference
basis is 1,000 ICP; account and range bases remain 10 and 20 ICP. The
high-activity multiplier was corrected from an impossible 125% to 100%.
Multipliers must lie in 1..=10,000 basis points, and each admission class is
rounded independently to whole ICP: at 62.5%, the three minima are 7, 13, and
625 ICP. Patrons can voluntarily endow more than the qualifying minimum.
The default funding calculation assumes exact-minimum patron endowments; extra
principal is not represented by an inadmissible multiplier above 100%.

The planned 3,000 ICP productive protocol endowment is now assigned to each
canonical workload; the independent generic instance has no protocol seed and
receives payouts only from its own subscriber endowments. The stress workload
decouples subscriber counts from subscription backing but retains the
canonical protocol seed. Productive principal is never credited as liquid
cycles. Only assumed maturity-derived Faucet payouts enter each instance's
modelled balance.

## Cost profiles and maintenance

The corrected model retains separate reader profiles:

| Architecture | Empty poll | Residual block | Additional page |
|---|---:|---:|---:|
| Canonical ICP shared scan | 22,069,775 | 209,200 | 6,624,846 |
| Generic ICRC-3 plus ICP admission | 28,238,603 | 268,939 | 6,188,199 |

Page, block, decoding, and state costs remain partially confounded. These are
PocketIC coefficients, not universal replica prices.

Recurring maintenance is charged as independent scheduled work and never
folded into ordinary polls:

- hourly funding maintenance: 21,220,513 cycles/run, estimated from the
  Economy maintenance-inclusive median minus the isolated poll coefficient;
- daily pricing maintenance: 16,619,093 cycles/run, estimated from the
  integrated Fast all-lane residual after removing polling and funding.

These are qualified residual estimates rather than independently isolated
lane measurements. Over 365 days they contribute approximately 0.191 T cycles
in the corrected expected-workload runs.

## Corrected results

At each policy's own Continuous entry, no funding, and the expected workload:

| Policy | Entry/exit | 1 s | 2 s | 5 s | 10 s |
|---|---:|---:|---:|---:|---:|
| Current | 100/60 T | 16.51 d | 32.45 d | 77.14 d | 142.65 d |
| Former 125 T candidate | 125/60 T | 26.82 d | 52.72 d | 125.36 d | 231.80 d |
| Conservative | 160/90 T | 28.89 d | 56.78 d | 135.00 d | 249.63 d |
| Performance-oriented | 75/45 T | 12.38 d | 24.33 d | 57.86 d | 106.98 d |

No candidate meets the provisional 30-day Continuous objective at the
one-second assumption. At two seconds the current policy already exceeds 30
days. Because the effective production completion rate is not yet measured,
the local evidence cannot justify choosing 125 T or another replacement.

Under illustrative weekly payouts, the current policy's annual sustainability
also changes with the assumed Continuous interval: it is not average
sustainable at one or two seconds, and is sustainable at five and ten seconds
under the configured funding and traffic assumptions. This is sensitivity,
not a mainnet prediction.

Equal-balance comparisons start every policy at 75, 100, 125, and 160 T. At
100 T, for example, the current policy immediately enters Continuous and
spends 16.51 of the 90 days there; both the 125 T and conservative candidates
remain Very Fast for the full 90 days. The performance-oriented candidate
spends 22.70 days in Continuous but retains less balance. Raising an entry
threshold therefore trades responsiveness for protection; it does not create
runway from the same resources.

The current lower-tier entry-to-exit results for the expected workload are:

| Tier | Canonical runway | Generic runway | 30-day objective |
|---|---:|---:|---|
| Economy | 45.17 d | 36.62 d | Both meet |
| Standard | 70.35 d | 58.22 d | Both meet |
| Fast | 68.73 d | 58.99 d | Both meet |
| Very Fast | 35.66 d | 29.51 d | Generic misses narrowly |

Economy, Standard, and Fast build substantial backlog at one block per second,
so their balance runway alone is not evidence for lowering thresholds. The
mixed architecture and service results do not justify changing any lower-tier
threshold.

The generic instance's annual weekly-payout assumption fell from 445.84 T to
117.20 T when the incorrectly inherited canonical seed was removed. Its
projected annual expenditure also fell from 465.21 T to 159.11 T as the lower
balance selected slower modes. Correcting the high-activity admission
multiplier from 125% to 100% reduced its assumed annual payouts from
1,054.36 T to 909.22 T; the resulting mode history reduced projected global
notifications from 98.19 million to 83.83 million. Burst coalescing changed
notification counts and backlog projections where activity was concentrated
in fewer polls. Expected canonical Continuous entry-to-exit runway changed
by less than the displayed 0.01 day because one block per poll already matched
the former aggregate assumption. The PocketIC coefficients and raw scenarios
did not change.

Interruption comparisons reached no Reserve Protection event from their own
candidate entry balances, but produced materially different minima and
responsiveness. Stress workloads remained unsustainable and accumulated large
backlogs for every candidate. Rolling seven-day transition counts remained
bounded; the raw 50-case PocketIC hysteresis parity result is unchanged.

## Conclusion

The corrected model does not identify a static candidate that dominates the
current policy across poll-rate sensitivity, equal-balance responsiveness,
funding interruptions, backlog safety, reserve outcomes, and hysteresis.

The evidence-based recommendation is therefore **no production threshold
change yet**. This is a provisional retention decision, not proof that the
current table is optimal. Mainnet observation of the effective Continuous
completion rate, actual cycle burn, reader response sizes, traffic, fan-out,
and Faucet receipts is required before reconsidering the thresholds.

## Reproduction

```bash
./tools/scripts/calibrate-cadence smoke --seed 20261011 \
  --output observations/cadence/smoke-final-20261012
./tools/scripts/calibrate-cadence model \
  --input observations/cadence/full-20261011/measurements.json \
  --config tests/cadence-calibration/fixtures/model-config.json \
  --output observations/cadence/final-20261012 \
  --seed 20261011
./tools/scripts/calibrate-cadence report \
  --input observations/cadence/final-20261012
```

## Validation gates

The following final-source gates completed successfully:

- `cargo test -p event-horizon-cadence-calibration --locked`: 24 tests;
- `./tools/scripts/calibrate-cadence smoke ...`: 9 complete scenarios and 50 threshold observations;
- corrected full model and report: 205 simulations;
- `cargo fmt --all -- --check`;
- `cargo clippy --locked --workspace --all-targets -- -D warnings`;
- `cargo run -p xtask -- test_unit`: 5 suites, 142 passed tests, 106 intentionally ignored PocketIC tests;
- `POCKET_IC_MUTE_SERVER=1 DFX_IDENTITY=codex_local cargo run -p xtask -- test_all`: 111 suites, 248 passed tests, including all 106 PocketIC scenarios;
- `./tools/scripts/security-scan`: no unfiltered finding;
- source-manifest generation and verification.

Clean-commit canonical release checks are recorded in the final handover; the
source revision for those checks is created only after this validation report
and the final source manifest are staged.
