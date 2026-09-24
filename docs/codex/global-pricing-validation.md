# Global subscriptions and dynamic pricing validation

## Identity

- Starting commit: `c48778d1744f3b645d0165cfdccec2ee704db71c`
- Permanent baseline tag: `event-horizon-core-validated`
- Branch: `codex/global-pricing`
- Final commit: the handoff `HEAD` containing this report and the regenerated root manifest; its exact SHA is reported with the archive because a Git commit cannot contain its own identifier.
- No mainnet deployment, `X` alias publication, or controller removal was performed.

## Behavioral changes

- Added a principal-only global declaration stored directly in stable memory.
- A completed poll that processed any transaction incorporates global subscribers once, after reaching its pinned boundary.
- `poke([])` means a global-only match. Sorted account hints take precedence and remain one poke per subscriber per poll.
- Added CMC ICP/XDR observation, bounded rolling history, integer pricing, monthly freezes, stale carry-forward, and admission-time enforcement.
- Added the sole production application query `get_pricing`.
- Added a same-origin frontend pricing proxy and builder support for all three declaration forms and scheduled increase advice.
- Kept Ledger polling, archives, pooled funding, CMC conversion, cadence thresholds, and subscriber reconciliation semantics unchanged.

## Stable migration

Existing stable memory IDs and encodings remain unchanged: metadata/cursor at 0, account subscriptions at 1, and CMC conversion state at 2. The debug-only configuration remains at 3. Additions are the global set at 4, UTC-day observation map at 5, and pricing state cell at 6.

The committed fixture `tests/fixtures/event_horizon_core_validated_c48778d_debug.wasm` was built from the exact starting commit and has SHA-256 `db467c173b4e4b4d121b8447a7a6c474873a6a5ad0d6b6ac8762e8edd4b9d661`. PocketIC installed it, established a cursor, account admission, polling mode, and `TransferPending` CMC state, upgraded to the new Wasm, and proved those values survived. Global storage began empty, pricing remained uninitialized until the first successful observation, and repeated scheduler startup retained exactly three timer slots.

## Pricing definition

The retained window is the current UTC day bucket plus the preceding 1,460 buckets, for at most 1,461 successful daily observations. Expiry is duration based. Only one successful observation is stored per UTC date; missing dates create no records.

For retained floor `F` and selected latest rate `C`:

```text
account_icp = ceil(10 × F / C)
global_icp  = ceil(100 × F / C)
```

Both use checked `u128` intermediate arithmetic and are calculated independently. The first success initializes both floor and latest, yielding 10/100 ICP.

Effective boundaries are first-of-month 00:00:00 UTC. Freeze is exactly seven 86,400-second days earlier. Observations with `recorded_at < freeze_at` participate; an observation at or after freeze waits for a later epoch. A latest eligible CMC timestamp more than seven 86,400-second days old causes current prices to carry forward. Admission compares the current price at evaluation time.

## Production Candid

```candid
type Price = record { account_icp : nat64; global_icp : nat64 };
type Pricing = record {
  initialized : bool;
  current : Price;
  current_effective_at : nat64;
  next : opt Price;
  next_effective_at : nat64;
  next_freeze_at : nat64;
  observed_floor_xdr_permyriad : nat64;
  floor_observed_at : nat64;
  latest_xdr_permyriad : nat64;
  latest_observed_at : nat64;
  next_carried_forward_due_to_stale_rate : bool;
};
service : () -> { get_pricing : () -> (Pricing) query; }
```

## Commands and results

Executed from `/home/codexdev/src/Event_Horizon`:

- `npm ci` — passed; lockfile was already current.
- `cargo run -p xtask -- check` — passed: static checks, workspace build, 30 backend Rust unit tests, 5 frontend Node tests, zero failures.
- `cargo run -p xtask -- pocketic` — passed: 19/19 PocketIC tests.
- `DFX_IDENTITY=codex_local cargo run -p xtask -- local-smoke` — passed: 19/19 PocketIC tests.
- `cargo run -p xtask -- security` — passed: 0 vulnerabilities, 4 documented allowed unmaintained warnings, cargo-deny advisories/bans/licenses/sources passed, OSV reported no issues.
- `cargo run -p xtask -- release` — passed; backend and frontend export audits passed and artifact checksums verified.
- `./tools/scripts/docker-build` — canonical Docker build passed and its manifest verified.
- `./tools/scripts/verify-reproducible-artifacts` — passed: `release artifacts match across two clean builds`.
- `DFX_IDENTITY=codex_local icp build -e local` — passed: `Canisters built successfully`.
- `./tools/scripts/source-manifest generate && ./tools/scripts/source-manifest verify` — passed after this report and `summary.txt` were committed.
- `python3 tools/audit-wasm.py release-artifacts/event_horizon.wasm --backend` — covered by release and canonical builds; passed with 9 total Wasm exports and exactly `canister_query get_pricing` as the application surface.

Canonical Docker Wasm SHA-256:

- backend: `e71d53a4748959cdd0ad9d0d71ad0e4dd74d699827f842dedd7977cf73a0a82d`
- frontend: `83aa275ec24a13c3ec05a2bd8c6805564454f5f2ad2343c0158d825030798def`

## Cost and scaling observations

PocketIC reported `Call::get_cost()` of `42,102,445,000 cycles` for one bounded daily CMC pricing query. This is the reserved upper-bound call cost in that environment, not measured net mainnet burn after unused response reservation refunds.

At the conservative stable value bound of 128 bytes, 1,461 observation values occupy at most 187,008 bytes before stable-map overhead. Global registry iteration is O(G) once per poll that processes transactions, not once per transaction. Such a poll can add at most one poke attempt per admitted global subscriber. No subscriber allowances or priority tiers were added.

## Coverage decisions and limitations

- Pinned-boundary interleaving was achieved with a real asynchronous PocketIC poll suspended at the existing Historian await. A transaction appended after the first Ledger response was neither processed nor poked until the following poll.
- Independent certificate/witness verification was not added. The existing PocketIC test verifies the `IC-Certificate` header and reconstructs the complete supplied SVG through certified range responses. Adding an independent verifier would require substantial agent/root-key test setup and dependencies solely for this optional item.
- The frontend's dynamic pricing JSON is a consensus HTTP update response; static frontend assets retain their existing certificate tree.
- The observed floor is Event Horizon's lowest successful daily sample, not an actual market low. Missing samples are accepted and never backfilled.
- Pricing begins only after the first successful post-install or post-upgrade CMC observation.
- No archive traversal, Index dependency, delivery retry, administrative recovery, or transaction search was added.
- Existing extraordinary CMC transfer ambiguity and history-gap limitations remain as documented in the prior post-validation handover.
