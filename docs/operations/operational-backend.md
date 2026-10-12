# Operational backend

## Poll lifecycle

A first successful zero-length Ledger read bootstraps prospectively. Each later poll pins the first response's exclusive `chain_length`, processes only that interval, and persists page progress. Proven archived prefixes are logged and skipped without archive calls. A legacy empty `blocks` vector gives no meaning to `first_block_index`; only explicit contiguous archive ranges can advance it. Partial evidence advances only its covered prefix, and a response below the pinned boundary with neither live blocks nor usable evidence fails without moving the cursor.

Each transfer performs one direct watched-account lookup. Qualifying matches accumulate as `subscriber -> WatchTarget -> maximum qualifying Nat`, with deterministic target ordering and a 256-target per-poke bound. If at least one transaction was processed, the backend adds admitted global subscribers once after reaching the boundary. It sends at most one target-aware poke per subscriber: a non-empty specific-target vector wins; otherwise a global match sends an empty vector.

Public logs are intentionally episode-based. A repeated reader failure may be
suppressed until a successful scan clears that failure episode. A daily
`HEALTH` line proves that maintenance ran, not that either reader completed or
is caught up. Cursor growth may include explicitly skipped archived blocks,
and Economy mode or downtime can still produce genuine archive skips; this
service does not become lossless merely because the empty-page interpretation
is corrected.

For post-upgrade acceptance, compare a later reader cursor with a read-only
Ledger-tip observation taken near the same time, while accounting for the
Ledger continuing to grow. Do not require samples from different instants to
be equal, and do not treat disappearance of one error line as proof of
recovery. Correlate cursor progress, the nearby tip, fresh subscriber behavior,
and any new or suppressed failure episode.

## Admission

Only a payout from the configured Faucet default account to Event Horizon can
propose admission; the patron's original staking transfer cannot. Historian
must verify the exact memo route, a complete view, and a cumulative qualifying
productive-endowment total at least equal to the current account/range/global
requirement. Pricing must have initialized. Repeated exact-route endowments may
raise that total, but cumulative principal is not cumulative rewards
distributed. A valid ordinary or neuron range expands inclusively into at most
256 watched-account merges. Failed evaluation creates no retry queue; a later
Faucet payout is another opportunity. Admission is permanent and extra funding
creates no priority or faster cadence.

## Pricing lane

An independent one-shot timer handles daily CMC observations and exact freeze/effective timestamps. It prunes observations older than the 1,461 UTC-day window before epoch work, records no more than one success per UTC date, and persists one attempt per date to avoid same-day retry loops. Upgrade startup replaces timer slots and resumes this lane once.

A fresh pricing state starts from 10 ICP account, 20 ICP range, and 1,000 ICP
global reference bases after its first successful CMC observation. The range
requirement remains the independent 20 ICP basis even though the stable
`Price` stores only account and global values. Old 100 ICP global pricing state
must not cross this policy boundary through an ordinary upgrade; the
exceptional prelaunch decision and destructive-reset preconditions are in the
[deployment runbook](deployment.md).

## Polling cadence

`T = 10^12 cycles`. The cadence input is the canister's **liquid cycles**: cycles immediately available to spend after outstanding-call reservations, not the larger total balance that may include reserved call balances.

| Mode | Added delay after completed poll | Enter at | Exit below |
| ------------------ | -------------------------------: | -------: | ---------: |
| Reserve Protection | ordinary polling suspended | — | 1 T |
| Economy | 1 hour | 2 T | 1 T |
| Standard | 10 minutes | 5 T | 3 T |
| Fast | 2 minutes | 10 T | 6 T |
| Very Fast | 10 seconds | 25 T | 15 T |
| Continuous | 0 | 100 T | 60 T |

**Threshold rationale.** Graduated delays and hysteresis balance responsiveness with cycles resilience. The 1 T Reserve Protection floor suspends ordinary polling while funding recovery remains available. Local PocketIC calibration measured about 22 million cycles per empty canonical ICP poll, 28 million for a generic ICRC-3 instance, and 5.5 million marginal cycles per subscriber notification. In the illustrative expected workload, the 100 T Continuous entry and 60 T exit provide about 16.5 days without funding at an assumed one completed poll per second, or 32.5 days at two seconds. Slower tiers reduce burn but may accumulate Ledger backlog; archived records can become unavailable through the chosen reader contract. Because effective mainnet completion frequency and recurring Faucet payouts remain uncertain, no evaluated static alternative consistently outperformed the current policy. These thresholds are retained provisionally pending operational calibration. See the [methodology](../development/cadence-calibration.md) and [validation evidence](../codex/cadence-calibration-validation.md).

Hysteresis prevents oscillation at entry thresholds. Event Horizon increases cadence immediately when a higher entry threshold is reached and may jump directly to the fastest enterable mode. When cycles fall, it remains in the current mode until that mode's lower exit threshold is crossed, then falls back as far as necessary. A canister at 5.27 T starting from a lower mode enters Standard mode. It remains Standard while its liquid balance is at least 3 T. Below 3 T it falls back to an appropriate lower mode.

These durations are **added delays after completed polls**, not exact poll intervals. Each poll performs its Ledger and inter-canister work before the next one-shot timer is scheduled. A single-flight guard prevents overlapping polls. Continuous means Event Horizon deliberately inserts no delay after a completed poll; polls still never overlap, and asynchronous IC execution naturally yields between calls.

Every scheduled polling callback refreshes the persisted mode from the current liquid balance before entering the reader. If the refreshed mode is Reserve Protection, it performs no ordinary Ledger/profile/admission read, rearms the one-hour reserve recheck, and returns. This entry check preserves hysteresis: an instance already in Economy may still poll at 1.5 T, while an instance in Reserve Protection must reach the 2 T Economy entry threshold before polling resumes. Below 1 T ordinary polling remains suspended, but the independent funding and pricing/recovery lanes remain scheduled. Per-call reserve checks remain in the Ledger clients because liquid balance can change after a poll begins.

Each of the polling, funding, and pricing one-shot wrappers owns its existing single-flight flag through all awaits. With the pinned Rust CDK, a post-await continuation trap cancels and drops that future. A scheduler-local cleanup guard then releases only the lane it acquired and, only when no legitimate replacement timer exists, installs one delayed attempt using the existing one-hour reserve-recheck interval. Normal cadence is unchanged. The cleanup performs no inter-canister call and rewrites no funding state; the later invocation resumes ordinary logic, including persisted transfer identities. PocketIC tests cover injected post-await traps in all three real wrappers, same-day pricing-attempt behavior, funding recovery from `CmcNotifyPending`, and overlapping ownership. This is tested cancellation recovery, not a claim of recovery from out-of-memory, invalid stable state, or every synchronous trap.

A successful CMC mint immediately reevaluates cadence from the newly available liquid cycles instead of waiting for the next completed poll. This polling table is independent of adaptive surplus policy: Continuous begins at 100 T, while 150 T is a surplus-health and transfer gate used only when the instance configuration enables a surplus destination.

## Funding lane

Hourly funding remains separate and is the only money-moving worker. Productive
endowment principal remains in Jupiter's staking path; Event Horizon receives
raw-ICP maturity-derived payouts and pools them with other eligible incoming
raw ICP. Each opportunity records the current liquid balance into the surplus
policy's seven-day observed minimum, then resumes the single durable
`FundingState` before considering any new raw balance. A successful mint
immediately recalculates polling cadence from liquid cycles. Pricing queries do
not alter funding state. Neither payout amount nor cadence is guaranteed.

With `InstanceConfig.surplus_canister = None`, the policy resets to level zero and the legacy single CMC transfer is unchanged. When an immutable recipient is configured and liquid cycles are currently at least 150 T, a new balance may be split after reserving two current Ledger fees. The exact retained amount, surplus amount, fees, destination account identifier, and fixed memo are persisted before the retained legacy transfer. CMC `Processing` keeps the notification and plan pending. Recovery never re-reads instance configuration to redirect a persisted plan. Refund or terminal error cancels the planned surplus.

Only successful CMC minting permits the next step. The lane re-reads liquid cycles; below 150 T it cancels the surplus leg. Otherwise it adds a deterministic creation time and sends the surplus transfer using only the already-persisted account, memo `SURPLUS1`, amount, fee, and time. It never reconstructs a pending destination or memo from current configuration. Acceptance/duplicate clears the state, ambiguous outcomes retry the same identity, clean/no-debit outcomes replan later, and an expired identity logs once and clears. New ICP arriving during any pending state waits for a later plan.
