# Deployment

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

Both canisters are newly created and empty. Their first deployment must use explicit install mode,
`--mode install`, never reinstall. Install the backend first, verify its module and application surface, and
only then install the frontend. The operator performs these mainnet actions manually; the repository does not
provide an automatic deployment script.

## Operator pre-deployment gate

Before any mainnet install, install the locked frontend dependencies and run the full, intentionally expensive gate:

```bash
npm ci
cargo run -p xtask -- validate
```

Successful validation ends with the canonical Docker build, prints both uncompressed Wasm hashes, and leaves:

```text
release-artifacts/event_horizon.wasm
release-artifacts/event_horizon_frontend.wasm
```

Review the hashes before continuing. Do not deploy a locally compiled replacement while claiming the canonical hash.

## Deploy canonical artifacts

Canonical-artifact mode verifies `release-artifacts/release-artifacts.sha256`, does not rebuild, prints the exact artifact hash being handed to `icp deploy`, and copies that Wasm to the CLI-requested output path.

Install and verify the backend first:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon -e ic --mode install
```

After checking its live module hash and production application surface, install the frontend:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon_frontend -e ic --mode install
```

The recommended lifecycle is:

```text
validate
→ review printed hashes
→ inspect canister mapping/settings
→ install backend
→ compare live module hash
→ verify backend
→ install frontend
→ compare live module hash
```

Event Horizon releases uncompressed `.wasm` files, so each SHA-256 printed by `validate` and the deployment build helper is intended for direct comparison with the corresponding installed mainnet module hash. See [Reproducible builds](reproducible-builds.md) for independent manifest verification.

## Required backend settings

The production constructor is exactly `record { observed_ledger : principal }`. Canonical ICP selects the fixed legacy `query_blocks` adapter; any other principal selects the generic ICRC-3 adapter and must pass ICRC-1/ICRC-3/`1xfer` readiness. This is immutable dispatch, not fallback.

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

Jupiter Faucet endowments are recurring funding and should not be confused with immediately spendable cycles.

Account, range, and global admission requirements come from `get_pricing`; they begin at 10, 20, and 100 ICP after the first successful CMC observation. Those pooled endowments do not replace the initial cycle balance.

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

## Observation period

Controller removal is a separate operational decision. During the controlled observation period verify at least:

1. the live Ledger reader remains caught up;
2. no unexplained `HISTORY_GAP` appears under normal operation;
3. ICP-to-cycles conversion regularly returns to `Idle` without operator intervention;
4. the expected polling mode follows the public cycles balance;
5. public logs remain sparse enough to be useful in a 4 KiB rolling buffer;
6. honest subscribers receive prompt pokes while their independent reconciliation remains sufficient when pokes are absent;
7. actual cycle burn is compatible with the starting threshold table;
8. the installed backend module hash matches the canonical reproducible artifact.
9. daily CMC observations, seven-day freezes, month activation, and stale carry-forward behave as specified;
10. the production export audit reports exactly `get_instance` and `get_pricing` as application methods;
11. global and range subscription poke volume is sustainable at the observed registry size, including a validated 256-account maximum range.
12. the frontend export audit reports only `http_request`, and direct browser instance/pricing reads use the selected reviewed static-registry backend principal compiled into the certified frontend artifact;
13. low-cycle tests confirm daily pricing observation skips preserve the reserve without affecting core polling or funding maintenance.
14. Current-schema `FundingState` upgrade/recovery tests pass for retained transfer, CMC notify, and surplus transfer pending states.
15. if surplus is enabled, observed policy transitions, retained-first ordering, both 150 T gates, and the immutable destination account have been verified.

Only after that evidence is satisfactory should `docs/controller-removal.md` be followed.

## External dependencies

Production constants are compiled into the backend Wasm:

- ICP Ledger: `ryjl3-tyaaa-aaaaa-aaaba-cai`
- CMC: `rkp4c-7iaaa-aaaaa-aaaca-cai`
- Jupiter Historian: `j5gs6-uiaaa-aaaar-qb5cq-cai`
- Jupiter Faucet: `acjuz-liaaa-aaaar-qb4qq-cai`
- Surplus receiver: currently `None` (diversion disabled)

The surplus receiver is a compile-time trust anchor, not deployment input. Before a production build intended to enable diversion, replace `SURPLUS_CANISTER = None` with exactly one reviewed `Some("<principal>")`, rebuild the canonical Wasm, rerun every validation and export audit, and record the new hash. Never add a runtime setter. Once controllers are removed, the destination cannot change.

The Jupiter Faucet `X` alias must resolve to the actual deployed Event Horizon backend before subscription endowments are created.

## Frontend backend binding

The certified frontend embeds `eo6ei-gaaaa-aaaar-qchra-cai` directly in its JavaScript asset and therefore in
the frontend Wasm. Its no-argument production pricing path uses `https://icp-api.io` and the mainnet root key
embedded by the JavaScript agent. It does not discover the backend from cookies, canister environment values,
URL parameters, local storage, HTTP configuration, init arguments, or mutable canister state. The frontend's
own principal is deployment identity only and is not compiled into executable code.
