# Deployment

Event Horizon has two independently controlled canisters:

- `event_horizon`: autonomous trigger backend, intended eventually to have **no controllers**.
- `event_horizon_frontend`: certified informational frontend, intentionally left governable/upgradeable.

## Build and validate

Before any mainnet install:

```bash
npm ci
cargo run -p xtask -- check
cargo run -p xtask -- pocketic
cargo run -p xtask -- security
cargo run -p xtask -- repro
```

The canonical build is produced by `Dockerfile.repro` and appears in `release-artifacts/`.
Do not deploy a locally compiled replacement while claiming the canonical hash. To make `icp deploy`
consume the already verified canonical artifacts, first run:

```bash
./tools/scripts/docker-build
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 icp deploy -e ic
```

## Required backend settings

`icp.yaml` declares the production observability settings:

- `status_visibility: public`
- `log_visibility: public`
- `log_memory_limit: 4096`

These settings must be verified on mainnet before controller removal. Public status is the intended source
for current cycles balance, running state and module hash; `get_pricing` is not a status API.

## Initial funding

Provide an initial cycles balance large enough to keep the backend comfortably out of reserve protection during
its observation period. Jupiter Faucet endowments are recurring funding and should not be confused with immediate
spendable cycles.

Account, range, and global admission requirements come from `get_pricing`; they begin at 10, 20, and 100 ICP after the first successful CMC observation. Those pooled endowments do not replace the initial cycle balance.

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
10. the production export audit reports only `get_pricing` as an application method;
11. global and range subscription poke volume is sustainable at the observed registry size, including a validated 256-account maximum range.
12. the frontend export audit reports only `http_request` and direct browser pricing reads succeed using the injected backend canister ID;
13. low-cycle tests confirm daily pricing observation skips preserve the reserve without affecting core polling or funding maintenance.

Only after that evidence is satisfactory should `docs/controller-removal.md` be followed.

## External dependencies

Production constants are compiled into the backend Wasm:

- ICP Ledger: `ryjl3-tyaaa-aaaaa-aaaba-cai`
- CMC: `rkp4c-7iaaa-aaaaa-aaaca-cai`
- Jupiter Historian: `j5gs6-uiaaa-aaaar-qb5cq-cai`
- Jupiter Faucet: `acjuz-liaaa-aaaar-qb4qq-cai`

The Jupiter Faucet `X` alias must resolve to the actual deployed Event Horizon backend before subscription endowments are created.
