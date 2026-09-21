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
for current cycles balance, running state and module hash; Event Horizon deliberately exposes no status API.

## Initial funding

Provide an initial cycles balance large enough to keep the backend comfortably out of reserve protection during
its observation period. Jupiter Faucet endowments are recurring funding and should not be confused with immediate
spendable cycles.

The planned protocol baseline is approximately 100 ICP endowed to Event Horizon through Jupiter Faucet, plus the
10 ICP requirement for each subscription declaration. Those endowments do not replace the initial cycle balance.

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

Only after that evidence is satisfactory should `docs/controller-removal.md` be followed.

## External dependencies

Production constants are compiled into the backend Wasm:

- ICP Ledger: `ryjl3-tyaaa-aaaaa-aaaba-cai`
- CMC: `rkp4c-7iaaa-aaaaa-aaaca-cai`
- Jupiter Historian: `j5gs6-uiaaa-aaaar-qb5cq-cai`
- Jupiter Faucet: `acjuz-liaaa-aaaar-qb4qq-cai`

The Jupiter Faucet `X` alias must resolve to the actual deployed Event Horizon backend before subscription endowments are created.
