# Pre-mainnet hardening validation

## Revision and scope

- Starting commit: `774b2906fc752590292316aaf7704c690aa7d3fb`.
- Final commit: the handoff `HEAD` containing this report, the regenerated source manifest, and `summary.txt`; its exact SHA is reported with the archive because a commit cannot contain its own identifier.
- No subscription semantics, pricing formula, epoch rules, admission rules, polling behavior, CMC funding behavior, or stable-memory IDs/encodings changed.
- No mainnet deployment, Jupiter Faucet `X` publication, or controller removal was performed.

## Changes

- The daily CMC pricing lane now constructs the bounded `get_icp_xdr_conversion_rate` call, reads that exact call's current `get_cost()`, and issues that same call only when `canister_liquid_cycle_balance() >= RESERVE_PROTECTION_CYCLES + call_cost`. Saturating addition fails closed on overflow.
- The UTC day's attempt is persisted before the reserve check. A reserve-protected skip therefore causes no retry loop and leaves observations, prices, Ledger polling, CMC funding recovery, and admitted subscriptions unchanged. Existing stale-rate carry-forward applies naturally.
- The frontend `/pricing.json` upgrade proxy and `http_request_update` export were removed. The certified embedded JavaScript bundle uses pinned `@icp-sdk/core` 6.1.0 to read ICP CLI's `PUBLIC_CANISTER_ID:event_horizon` deployment value from the standard `ic_env` cookie and call the backend `get_pricing` query directly. No backend ID is hardcoded.
- The Rust frontend still embeds and certifies all static assets. It adds the deployment discovery cookie to the document response when the ICP CLI environment value is present. Backend `get_pricing` remains the protocol-authoritative display value, and backend admission remains authoritative.
- Recorded CMC rates are formatted in the browser by integer/string operations as exactly four decimal places in XDR/ICP; for example, `23147` renders as `2.3147 XDR/ICP`. Protocol arithmetic remains integer-only.
- The canonical release build now regenerates the pinned browser bundle before compiling the frontend Wasm. Static and Wasm audits assert that the removed update proxy cannot return.

## Deterministic coverage

- Backend Rust unit tests: 31 passed, including exact reserve boundary and overflow behavior.
- Frontend Node tests: 8 passed, including direct discovered-backend `get_pricing`, exact four-decimal rendering, and absence of the update proxy.
- PocketIC integration tests: 20 passed. The reserve case began with insufficient liquid cycles, proved the CMC method was not invoked, replenished cycles on the same UTC day and proved no retry occurred, then advanced one day and proved exactly one successful observation initialized pricing.
- The certified ranged SVG reconstruction still passes. The same PocketIC test proves `/pricing.json` returns 404 and `http_request_update` is not exported.
- `local-smoke` independently executed the same 20 PocketIC cases: all passed.

The PocketIC CMC call-cost value used at the tested boundary is `42,102,445,000 cycles`. It is the call's reserved cost in that environment, not a measurement of net mainnet burn after response refunds.

## Executed evidence

| Command | Result |
|---|---|
| `npm ci` | Pass; 13 packages installed from the lockfile. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- check` | Pass; static checks, format, Clippy/build checks, 31 Rust unit tests, doc tests, and 8 frontend tests passed. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- pocketic` | Pass; 20/20 integration tests passed in 174.07 seconds. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- local-smoke` | Pass; 20/20 integration tests passed in 171.83 seconds. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- security` | Pass; RustSec found no vulnerabilities and four permitted unmaintained transitive warnings; cargo-deny passed; npm audit and OSV reported no issues after the four documented filters. |
| `DFX_IDENTITY=codex_local cargo run -p xtask -- release` | Pass; both Wasm audits and the local release artifact checksum manifest passed. |
| `./tools/scripts/docker-build` | Pass; canonical Docker artifacts and their checksum manifest verified. |
| `./tools/scripts/verify-reproducible-artifacts` | Pass; every release artifact matched byte-for-byte across two clean Docker builds. Earlier attempts failed before compilation because repeated clean builds filled the Docker overlay; reclaiming 12.52 GB of disposable build cache resolved the invalid-download symptom without changing the procedure. |
| `DFX_IDENTITY=codex_local icp build -e local` | Pass; both canisters built successfully. |
| `python3 tools/audit-wasm.py release-artifacts/event_horizon.wasm --backend` | Pass; 9 total exports, with exactly `canister_query get_pricing` as the application method. |
| `python3 tools/audit-wasm.py release-artifacts/event_horizon_frontend.wasm --frontend` | Pass; 7 total exports, with exactly `canister_query http_request` as the application method and no update proxy marker. |
| `./tools/scripts/source-manifest generate && ./tools/scripts/source-manifest verify` | Run after the final evidence and handover commit; result recorded below before archive creation. |

## Canonical artifacts

- Backend Wasm SHA-256: `3ff42f18c38fc126909080c0ae06043e6a7a572a7b399e684ace7f835a5c1f07`.
- Frontend Wasm SHA-256: `d0acc19fa6f6e4d6d76c9d147818827f4ec985306e240e13e72cc5b41275028f`.
- Backend production application surface: `canister_query get_pricing` only.
- Frontend production application surface: `canister_query http_request` only.

## Remaining limitations

- A reserve-protected pricing skip can make the latest observation stale; the specified carry-forward behavior deliberately preserves the current prices.
- The frontend canister remains mutable. Certified asset verification and the agent-authenticated backend query protect transport integrity, while backend admission remains the authority if frontend presentation is unavailable or changed.
- Browser pricing depends on deployment injection of `PUBLIC_CANISTER_ID:event_horizon`; a missing deployment value produces the explicit unavailable state rather than using a fallback ID.
- No live mainnet burn measurement, deployment, alias publication, or controller removal was performed.
- The four existing permitted unmaintained Rust transitive warnings remain: `backoff`, `instant`, `paste`, and `serde_cbor`, with their existing documented scopes.
