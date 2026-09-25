# Generic ledger instances validation

## Revision and scope

- Starting commit: `c0afddded408bb1c9963b235d91b32478963bb79`.
- Implementation/artifact revision: `22e62acf171824b5a658c17fa98a694226a4f60b`.
- Final evidence commit: the commit containing this report; its SHA is reported with the release archive because a Git commit cannot contain its own hash.
- Branch: `codex/generic-ledger-instances`.
- No mainnet deployment, reinstall, alias publication, controller change, or Jupiter Faucet repository change was performed.

Commits through the validated artifact revision:

1. `a602f05` Introduce immutable instance configuration
2. `550518d` Add generic ICRC-3 block decoder
3. `325e5d0` Split admission and observed ledger streams
4. `502f806` Add generic ledger PocketIC coverage and observability
5. `c737a70` Add verified multi-instance frontend registry
6. `3b60200` Add acceptance cycles observation tooling
7. `d47b0d9` Document immutable generic instance deployment
8. `bd38a5c` Prove same-Wasm and dual-ledger invariants
9. `681e650` Harden generic stream processing semantics
10. `8dcd9b7` Complete generic validation hardening
11. `22e62ac` Pin reviewed generic backend hash in registry

No decoder, flag, fixture, or migration path for the prior acceptance schema was added. Current-schema upgrade behavior remains covered.

## Production contract and stable state

The production constructor is:

```candid
type InitArgs = record { observed_ledger : principal };
service : (InitArgs) -> {
  get_instance : () -> (InstanceInfo) query;
  get_pricing : () -> (Pricing) query;
}
```

Anonymous and management principals are rejected. The observed Ledger is written once and has no mutation method. Production exports exactly `canister_query get_instance` and `canister_query get_pricing`; there are no application update/admin methods. `SURPLUS_CANISTER` remains `None`.

Stable memory is: ID 0 metadata (including independent admission/observed cursors and daily-health marker), ID 1 watched ICRC accounts, ID 2 immutable instance configuration/profile, ID 3 debug-only injected dependencies, ID 4 global subscribers, ID 5 price observations, ID 6 pricing state, ID 7 ICP funding state, and ID 8 surplus-policy state.

## Ledger protocol semantics

Readiness queries `icrc1_supported_standards`, `icrc1_symbol`, `icrc1_decimals`, and `icrc3_supported_block_types`. ICRC-1, ICRC-3, and `1xfer` are required; `2xfer` is optional. Symbol storage is limited to 32 UTF-8 bytes. The official `icrc-ledger-types 0.2.0` types are locked in `Cargo.lock`.

The narrow decoder recognizes `btype=1xfer`, `btype=2xfer`, and the backward-compatible absent-`btype`/`tx.op=xfer` form. It extracts only `from`, `to`, `amt`, and optional `memo`. An identified malformed transfer fails closed and preserves its live-page cursor. Unknown valid types become `Other` and count only as global live activity. Collection guards reject more than 256 returned live blocks or more than 256 archive requests in aggregate; generic values are not recursively traversed.

Fresh admission and observed streams bootstrap prospectively at their respective `log_length`. The first response pins the poll boundary. Archive callbacks are never invoked. Explicit contiguous archive prefixes are coalesced, logged, and skipped; unexplained holes preserve the cursor. Archive-only progress is not global activity. Live cursor changes commit once only after the complete returned page succeeds. Shared ICP mode logs gaps as `stream=shared`, performs one physical ICRC-3 page read, and synchronizes both cursors.

For distinct Ledgers the ordering is observed scan against the starting subscription set, ICP admission scan, then delivery of the already-determined match set. No timestamp-based cross-ledger ordering is invented. Either stream still runs when the other fails.

ICRC accounts are keyed by principal length, principal bytes, and effective 32-byte subaccount. `null` and an explicit zero subaccount normalize identically; numbered subaccounts 1–255 use the final byte. Thresholds are arbitrary-precision `Nat` observed-token units. The structural parser accepts canonical unsigned ASCII decimals only; admission applies the discovered decimals. Explicit zero, signs, exponents, shorthand `.1`, leading zeros, and excess fractional precision are rejected. Omission remains the zero sentinel for every incoming transfer.

Admission and funding always use the fixed canonical ICP Ledger. Faucet source/destination/memo matching, Historian evidence, pricing, legacy CMC transfer, and top-up notification remain ICP-based. The observed asset never funds cycles.

## Public verification and observability

`get_instance` returns the observed Ledger/profile and the compiled ICP Ledger, CMC, Faucet, Historian, and disabled surplus destination. Init emits:

```text
CONFIG instance=<principal> observed_ledger=<principal> icp_ledger=<principal> faucet=<principal> historian=<principal> cmc=<principal> surplus=none
```

Readiness emits the bounded profile. At most once per UTC day the maintenance path emits:

```text
HEALTH day=<day> observed_ledger=<principal> symbol=<symbol-or-pending> mode=<mode> liquid_cycles=<cycles> observed_cursor=<block> admission_cursor=<block> watched_accounts=<count> global_subscribers=<count> pricing_initialized=<bool>
```

Actual mode changes emit `POLL_MODE_CHANGE from=<mode> to=<mode> liquid_cycles=<cycles>`. Ledger failures and `HISTORY_GAP` lines name `observed`, `admission`, or `shared`. Public backend log retention is 16384 bytes and transaction contents are not logged.

`tools/scripts/mainnet-observe` is separate, read-only host tooling. It captures timestamp/revision, backend status/metrics/instance/pricing/logs, and frontend status/metrics under ignored `observations/`. Controller-only `canister_metrics` failures are recorded without aborting. `docs/acceptance-observation.md` documents cumulative counter deltas, cycles/day, category contribution, cadence, balance, cursor, and subscriber observations.

## Frontend and immutable listing

The certified static registry contains exactly:

- ICP: live, canonical, alias `X`, backend `eo6ei-gaaaa-aaaar-qchra-cai`, observed Ledger `ryjl3-tyaaa-aaaaa-aaaba-cai`, expected backend hash `0a2b83a113fcbaa7277844a72e2a51d8004169e4a44df9ee1b025ca37b84daeb`.
- IO: planned, canonical, intended alias `I`, with backend/Ledger/hash all `null`.

The selected live backend's `get_instance` result must match the registry and expose a profile before memo construction. Alias bytes are included in the 32-byte limit. Threshold labels/precision use the verified token profile, while prices remain ICP. There is no remote/mutable registry, cookie, URL, local-storage, or `ic_env` configuration.

Listing requirements cover a published dedicated Jupiter alias, ICRC-1/3 plus `1xfer`, matching query/log configuration, reviewed generic Wasm hash, public status/logs, adequate funding, reproducible source evidence, and `controllers=[]`. The verification tuple is canister ID + module hash + empty controllers + observed Ledger query/log evidence + Jupiter alias. IO identifiers were deliberately not fabricated.

## Test and release evidence

- `npm ci`: passed.
- `cargo run -p xtask -- validate` with `DFX_IDENTITY=codex_local`: passed.
- Backend unit tests: 50 passed.
- Frontend tests: 6 passed.
- PocketIC tests: 44 passed.
- Security gate: passed (`cargo audit`, `cargo deny`, npm audit, OSV policy).
- Two clean canonical Docker builds: byte-identical.
- Canonical backend SHA-256: `0a2b83a113fcbaa7277844a72e2a51d8004169e4a44df9ee1b025ca37b84daeb`.
- Canonical frontend SHA-256: `f853976bde532dfc6fc07f0585f1153e5a483c4bf9ccf72f34ba602172d3b35b`.
- Backend export audit: passed, exactly the two production queries.
- Frontend export audit: passed, exactly `canister_query http_request` as its application method.
- `DFX_IDENTITY=codex_local icp build -e local`: passed.

PocketIC installs the same cached backend Wasm bytes into instances with different observed Ledgers, verifies the identical hash/bytes, observes different `get_instance` values, proves each instance reacts only to its configured Ledger, and proves both use the ICP admission/funding Ledger. Further regression evidence covers prospective bootstrap, fixed boundaries, archive gaps without callbacks, atomic malformed-page replay, archive-only non-activity, ICRC-1/2 transfers, unknown global activity, specific/global precedence, coalescing, separate-stream failure independence, non-retroactive cross-ledger admission, one-fetch shared mode, pricing, funding, cadence, surplus-disabled operation, and current-schema upgrades.

Static checks prove the canonical ICP trust anchor is compiled into backend source, no production observed-instance principal is compiled into the backend, the production surface is query-only, and the frontend registry has only ICP live and IO planned. Reproducibility and artifact manifests passed. The source manifest is regenerated and verified after this report in the final evidence commit.

Remaining planned IO fields are its canonical SNS Ledger principal, Event Horizon backend principal, and expected generic release hash at launch. They remain intentionally unset pending launch, alias approval, verification, controlled testing, and controller removal.
