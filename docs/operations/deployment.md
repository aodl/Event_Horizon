# Deployment

This is the operator runbook for the canonical Event Horizon mainnet deployment.

Event Horizon has two independently controlled canisters:

- `event_horizon`: `eo6ei-gaaaa-aaaar-qchra-cai`, the autonomous trigger backend, intended eventually to have **no controllers**.
- `event_horizon_frontend`: `ej7c4-lyaaa-aaaar-qchrq-cai`, the certified informational frontend, intentionally left governable/upgradeable.

These permanent mainnet canisters have already been created by the operator. Before any mainnet deployment,
map the project names to the existing principals in the operator-local `.icp/data/mappings/ic.ids.json`:

```json
{
  "event_horizon": "eo6ei-gaaaa-aaaar-qchra-cai",
  "event_horizon_frontend": "ej7c4-lyaaa-aaaar-qchrq-cai"
}
```

This mapping is local deployment state and must not be committed as application source.

> **Do not run a mainnet deploy until the mapping has been checked. Otherwise a deployment tool may create new canisters rather than use the permanent Event Horizon canisters.**

The canonical ICP backend is already installed. This 1,000 ICP global-pricing
release is an exceptional prelaunch state-policy boundary: old stable global
prices cannot safely be interpreted using the new range divisor. The selected
strategy is a controlled reinstall, but only after the decision gate below is
completed and separately approved. Subsequent ordinary releases return to
state-preserving, argument-free upgrades. Reinstall must never be used to
discard a Ledger backlog. A genuinely new additional instance uses install
mode. The operator performs every mainnet action manually; the repository
provides no automatic deployment script.

## Frontend-only ledger-counter release

The landing ledger total is a browser feature. Only the separately controlled
frontend is a deployment candidate after release gates and separate operator
authorization. It requires no backend module, migration, settings change,
pricing reset, reinstall, transfer or controller action. Do not apply the earlier
exceptional backend pricing-reset procedure to this release. Compare the backend
canonical hash with its reviewed baseline and retain its existing registry pin.
No deployment is part of counter validation. See the
[validation report](../codex/ledger-counter-validation.md).

## Exceptional prelaunch pricing-reset decision

Before authorising reinstall, use read-only production records, current
Historian route evidence, public logs/status, and operator records to verify all
of the following:

- no subscription has been admitted and no patron has an outstanding
  expectation based on an existing endowment;
- no retained CMC transfer, CMC notification, surplus transfer, or other
  financial obligation needs preservation;
- loss of pricing observations/frozen prices, Ledger cursors, discovered
  profile, subscriptions, and all other stable state is explicitly accepted;
- the immutable constructor tuple remains canonical ICP Ledger, no SNS Root,
  and no surplus recipient;
- a separate liquid-cycles bootstrap is ready for the intended cadence while
  initial productive-endowment payouts are pending.

Production deliberately exposes no subscription/debug status query. If those
facts cannot be established from operational evidence, stop: this runbook does
not make reinstall safe and no migration framework is supplied. Obtain a
separate operator decision rather than upgrading old pricing state or assuming
the earlier zero-subscriber observation is still current.

## Operator pre-deployment gate

Before any mainnet install, run the behavioural, source-quality, security, reproducibility, and canonical artifact gates explicitly:

```bash
POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
./tools/scripts/security-scan
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
```

The test command prepares locked frontend dependencies automatically. The final canonical Docker build prints both uncompressed Wasm hashes and leaves:

```text
release-artifacts/event_horizon.wasm
release-artifacts/event_horizon_frontend.wasm
```

Review the hashes before continuing. Do not deploy a locally compiled replacement while claiming the canonical hash.

## Deploy canonical artifacts

Canonical-artifact mode verifies `release-artifacts/release-artifacts.sha256`, does not rebuild, prints the exact artifact hash being handed to `icp deploy`, and copies that Wasm to the CLI-requested output path.

The protective `--no-create` option requires `icp-cli` 1.2.0 or newer; it makes
deployment fail if the mapped canister does not exist instead of creating a
different canister. Confirm the installed version and `icp deploy --help`
before proceeding. If the option is unavailable, update or otherwise verify
the deployment tooling; do not silently omit the flag.

For this pricing-policy transition only, and only after the decision gate and
explicit approval, reinstall the existing canonical backend with the reviewed
constructor. Omitting `--yes` preserves the CLI's destructive-action
confirmation:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon -e ic --mode reinstall --no-create \
  --args-file canisters/event-horizon/mainnet-icp-install-args.did
```

This destroys all backend stable state. Preparing this release does not
authorise that action. After this exceptional reset has been accepted and
completed, ordinary backend releases take no constructor argument and use:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon -e ic --mode upgrade
```

After checking its live module hash, configuration, and production application
surface, inspect the permanent frontend canister immediately before acting. Use
the existing `event_horizon_frontend` mapping and check whether the canister has
an installed module. If it does, deliberately select upgrade:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon_frontend -e ic --mode upgrade
```

If the permanent frontend canister exists but has no installed module,
deliberately select install instead:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon_frontend -e ic --mode install
```

Do not assume either frontend state and do not automate this choice. The
ordinary backend instruction above remains an argument-free upgrade. The
exceptional reinstall is destructive and is not a backlog-recovery procedure.

For a fresh additional instance, use `--mode install` and follow
[Deploying an immutable instance](deploying-an-instance.md). For an SNS-aware
instance, that guide owns the Root/Ledger/Governance verification procedure.

The recommended lifecycle is:

```text
behavioural/source/security/reproducibility gates
→ canonical Docker build
→ review printed hashes
→ inspect canister mapping/settings
→ complete the pricing-reset decision gate
→ separately approve and perform the exceptional backend reinstall
→ wait for prospective reader/profile/pricing initialization
→ verify fresh 10/20/1,000 pricing and new cursor baselines
→ compare live module hash
→ verify backend
→ inspect frontend installed-module state
→ install or upgrade frontend as applicable
→ compare live module hash
```

Event Horizon releases uncompressed `.wasm` files, so each SHA-256 printed by the canonical build helper is intended for direct comparison with the corresponding installed mainnet module hash. See [Reproducible builds](reproducible-builds.md) for independent manifest verification.

## Required backend settings

The production constructor is:

```candid
record {
  observed_ledger = principal "ryjl3-tyaaa-aaaaa-aaaba-cai";
  sns_root = null;
  surplus_canister = null;
}
```

Canonical ICP requires `sns_root = null` and selects the fixed legacy
`query_blocks` adapter. Any other principal selects the generic ICRC-3 adapter
and must pass ICRC-1/ICRC-3/`1xfer` readiness. This is immutable dispatch, not
fallback. `surplus_canister = null` keeps canonical ICP CMC-only unless the
repository owner deliberately supplies a reviewed receiver before reinstall.
SNS-aware instance deployment is covered by
[Deploying an immutable instance](deploying-an-instance.md).

`icp.yaml` declares the production observability settings:

- `status_visibility: public`
- `log_visibility: public`
- `log_memory_limit: 16384`

These settings must be verified on mainnet before controller removal. Public status is the intended source
for current cycles balance, running state and module hash; `get_pricing` is not a status API.

## Initial funding

Initial funding is an operator choice, not a requirement to fund Continuous mode immediately. During controlled acceptance testing it is reasonable to deliberately operate at a lower tier to limit irreversible ICP-to-cycles conversion. Choose the initial cycles balance according to the latency tier you intend to observe.

`T = 10^12 cycles`. Cadence uses **liquid cycles**, excluding balances reserved for outstanding calls.

| Mode | Added delay after completed poll | Enter at | Exit below |
| ------------------ | -------------------------------: | -------: | ---------: |
| Reserve Protection | ordinary polling suspended | — | 1 T |
| Economy | 1 hour | 2 T | 1 T |
| Standard | 10 minutes | 5 T | 3 T |
| Fast | 2 minutes | 10 T | 6 T |
| Very Fast | 10 seconds | 25 T | 15 T |
| Continuous | 0 | 100 T | 60 T |

Typical entry targets are:

```text
~5 T   → Standard entry
~10 T  → Fast entry
~25 T  → Very Fast entry
~100 T → Continuous entry
```

Event Horizon accelerates immediately when it reaches a higher entry threshold. On the way down it stays in its current mode until that mode's exit threshold is crossed. For example, a canister at 5.27 T starting from a lower mode enters Standard and remains there while its liquid balance is at least 3 T; below 3 T it falls back to an appropriate lower mode.

Below 1 T, Reserve Protection suspends ordinary Ledger polling but leaves funding and other recovery-oriented maintenance available. A canister already in Reserve Protection must reach the 2 T Economy entry threshold before polling resumes. Continuous inserts no delay after a completed poll, but polls never overlap and asynchronous IC execution naturally yields between calls. These are added delays, not exact wall-clock poll intervals.

Higher balances may be appropriate once sustained low-latency service is desired. The 150 T surplus-health threshold is not a polling tier: Continuous polling starts at 100 T, while 150 T applies only to the currently disabled adaptive surplus policy.

Jupiter Faucet endowments are productive principal and should not be confused
with immediately spendable cycles. Principal remains in Jupiter's staking path;
recurring maturity-derived raw-ICP payouts enter Event Horizon and are converted
into the common cycles pool.

Account, range, and global admission requirements come from `get_pricing`; they
begin at 10, 20, and 1,000 ICP after the first successful CMC observation, then
follow the existing monthly dynamic policy. Those pooled payouts do not replace
the initial liquid cycle balance. The intended 3,000 ICP protocol endowment is
a planned productive contribution, not a confirmed deposit or liquid bootstrap.
Continuous-mode entry requires 100 T liquid cycles and exits below 60 T.

## Observe cycles and infer cadence

The production backend exposes public canister status. The simplest operational check is:

```bash
icp canister status eo6ei-gaaaa-aaaar-qchra-cai -n ic
```

The `Cycles:` field is the easiest public signal for estimating the active cadence tier. It is not an internal mode field, and `canister_status` does not expose Event Horizon's polling mode directly. Infer the likely tier from the current balance, the previous mode, and the hysteresis table above. Because the implementation decides from liquid cycles, outstanding-call reservations can make the immediately spendable input slightly lower than the reported balance.

## Read current pricing

With the project canister mapping configured, query readable pricing by name:

```bash
icp canister call event_horizon \
  get_pricing '()' \
  -e ic \
  --candid canisters/event-horizon/event_horizon.did
```

The raw-principal equivalent is:

```bash
icp canister call eo6ei-gaaaa-aaaar-qchra-cai \
  get_pricing '()' \
  -n ic \
  --candid canisters/event-horizon/event_horizon.did
```

When calling by raw principal without a local Candid interface, `icp-cli` may render record field hashes instead of field names. Supplying the checked-in DID gives readable named fields.

The response fields are:

- `initialized`: whether the first successful CMC rate observation has established pricing.
- `current`: current account, range, and global admission prices.
- `next`: an optional frozen price for the next effective month.
- `current_effective_at`: when the current prices became effective.
- `next_effective_at`: when `next` will become current.
- `next_freeze_at`: the seven-day boundary at which the next prices freeze.
- `observed_floor_xdr_permyriad`: the retained rolling-window floor CMC rate.
- `latest_xdr_permyriad`: the latest successful CMC rate observation.
- `next_carried_forward_due_to_stale_rate`: whether stale-rate protection carried current prices into the next period.

`next_carried_forward_due_to_stale_rate = true` can legitimately occur on first deployment when the next month's seven-day freeze boundary already passed before Event Horizon had its first successful rate observation. It is not a fault. The response also includes timestamps for the floor and latest observations.

## Post-reinstall ordering

After an explicitly approved reinstall, keep new endowments closed until all of
the following are observed in order:

1. the constructor tuple and installed backend hash match the reviewed release;
2. observed-ledger profile discovery succeeds;
3. the fresh prospective ICP reader has bootstrapped to its then-current tip;
4. the first successful CMC observation reports current prices of account 10,
   range 20, and global 1,000 ICP;
5. sufficient liquid cycles exist for the chosen acceptance cadence;
6. the rebuilt frontend's static registry contains the installed backend hash
   and displays the same authoritative prices.

Only then should new subscriber or protocol endowments be accepted. Reinstall
does not replay historical Faucet payouts, so an endowment sent before the new
prospective cursor was established might never provide the live payout event
required for admission.

## Observation period

Controller removal is a separate operational decision. During the controlled observation period verify at least:

1. the live Ledger reader remains caught up;
2. no unexplained `HISTORY_GAP` appears under normal operation;
3. ICP-to-cycles conversion regularly returns to `Idle` without operator intervention;
4. the expected polling mode follows the public cycles balance;
5. public logs remain sparse enough to be useful in the configured 16 KiB (`16384` byte) rolling buffer;
6. honest subscribers receive prompt pokes while their independent reconciliation remains sufficient when pokes are absent;
7. actual cycle burn is compatible with the starting threshold table;
8. the installed backend module hash matches the canonical reproducible artifact.
9. daily CMC observations, seven-day freezes, month activation, and stale carry-forward behave as specified;
10. the production export audit reports exactly `get_instance` and `get_pricing` as application methods;
11. global and specific-target poke volume is sustainable at the observed registry size, including validated 256-target range-expansion and callback bounds.
12. the frontend export audit reports only `http_request`, and direct browser instance/pricing reads use the selected reviewed static-registry backend principal compiled into the certified frontend artifact;
13. low-cycle tests confirm daily pricing observation skips preserve the reserve without affecting core polling or funding maintenance.
14. Current-schema `FundingState` upgrade/recovery tests pass for retained transfer, CMC notify, and surplus transfer pending states.
15. `get_instance.surplus_canister` and the CONFIG log match the reviewed install argument; if surplus is enabled, observed policy transitions, retained-first ordering, both 150 T gates, and the immutable destination account have been verified.
16. A patch upgrade retains the pre-upgrade nonzero reader cursors,
    subscriptions, pricing, immutable configuration, and pending financial
    state before ordinary catch-up resumes.

For reader-patch acceptance, take a read-only Ledger-tip observation near a
later cursor observation and compare the two with timestamps. The Ledger may
grow between samples, and cursor advancement can include explicitly skipped
archive ranges, so equality between differently timed observations is neither
required nor sufficient. Confirm fresh processing and failure-episode behavior
as well; do not validate the patch solely by the absence of one log line.

Only after that evidence is satisfactory should [controller removal](controller-removal.md) be followed.

## External dependencies

Wasm-level protocol constants are compiled into the backend Wasm:

- ICP Ledger: `ryjl3-tyaaa-aaaaa-aaaba-cai`
- CMC: `rkp4c-7iaaa-aaaaa-aaaca-cai`
- Jupiter Historian: `j5gs6-uiaaa-aaaar-qb5cq-cai`
- Jupiter Faucet: `acjuz-liaaa-aaaar-qb4qq-cai`

Instance installation persists `observed_ledger`, optional `sns_root`, and
optional `surplus_canister`. The receiver is deployment input, not a build
input: every recipient uses the same canonical backend Wasm. There is no
runtime setter or upgrade argument. Verify `get_instance` and the initial
CONFIG log against all three values; once controllers are removed, the entire
instance is immutable.

The Jupiter Faucet `X` alias must resolve to the actual deployed Event Horizon backend before subscription endowments are created.

## Frontend backend binding

The certified frontend embeds a reviewed static instance registry, including
canonical backend `eo6ei-gaaaa-aaaar-qchra-cai`, in its JavaScript asset and
therefore in the frontend Wasm. Its production query path uses
`https://icp-api.io` and the mainnet root key embedded by the JavaScript agent.
It calls `get_instance` to verify the selected registry entry's observed
Ledger, SNS Root, surplus recipient, and profile, and calls `get_pricing`
for authoritative prices. It does not discover principals from cookies,
canister environment values, URL parameters, local storage, HTTP configuration,
init arguments, or mutable canister state. The frontend's own principal is
deployment identity only and is not compiled into executable code.
