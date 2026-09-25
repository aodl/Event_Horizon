# Mainnet ID binding validation

- Starting commit: `a9a75afb3607a0c5ae2f9744d34785477e60511c`
- Final commit: the commit containing this report, titled `Document permanent mainnet identities` (use `git rev-parse HEAD` for its immutable SHA)
- Permanent backend: `eo6ei-gaaaa-aaaar-qchra-cai`
- Permanent frontend: `ej7c4-lyaaa-aaaar-qchrq-cai`

## Binding

`canisters/frontend/public/pricing-client.js` exports the permanent backend principal and
`https://icp-api.io` as explicit constants. The no-argument `queryBackendPricing()` path passes those values
to the mainnet agent and actor. `app.js` calls that no-argument path. The normal pinned esbuild process embeds
the principal in `app.bundle.js`; the Rust frontend embeds and certifies that asset in the canonical Wasm.

The frontend discovery-cookie response logic and its percent encoder were removed. The JavaScript client no
longer imports canister-environment discovery or accepts runtime cookie/root-key configuration. No replacement
runtime configuration mechanism was introduced.

## Tests and checks

- Frontend pricing tests now assert the exact default principal and host, actor routing, explicit test-only
  dependency injection, unchanged pricing normalization/rendering, and absence of a discovery cookie in the
  Rust implementation.
- `npm ci`: passed.
- `cargo run -p xtask -- check`: passed; 46 backend Rust unit tests and 13 frontend JavaScript tests passed.
- `cargo run -p xtask -- pocketic`: passed; 37 integration tests passed.
- `cargo run -p xtask -- local-smoke`: passed; 37 integration tests passed.
- `cargo run -p xtask -- security`: passed with the four existing documented/allowed warnings and zero
  unfiltered vulnerabilities.
- `cargo run -p xtask -- release`: passed.
- `./tools/scripts/docker-build`: passed.
- `cargo run -p xtask -- repro`: passed; artifacts matched across two clean no-cache builds.
- `DFX_IDENTITY=codex_local icp build -e local`: passed.

## Canonical artifacts

- Backend SHA-256: `d5a66897cb914488df53799842a89912f65a0688793b3eb198b064ed7d5c202b`
- Frontend SHA-256: `3f19289a19a940ed023d05957a24c3e2d0b50402ddfffd978f08daa7aa01df63`

The backend hash exactly equals the validated baseline hash. `git diff
a9a75afb3607a0c5ae2f9744d34785477e60511c -- canisters/event-horizon` is empty, independently proving that
backend source did not change.

The canonical frontend Wasm contains `eo6ei-gaaaa-aaaar-qchra-cai` twice. It contains zero instances of each
removed discovery marker: `PUBLIC_CANISTER_ID:event_horizon`, `ic_env`, and `IC_ROOT_KEY`. The active frontend
source and bundle also contain none of `safeGetCanisterEnv`, `PUBLIC_CANISTER_ID:event_horizon`,
`canister_discovery_cookie`, `percent_encode_cookie_value`, `ic_env`, or `IC_ROOT_KEY`.

The production Wasm export audit passed:

- Backend application surface: `canister_query get_pricing` only.
- Frontend application surface: `canister_query http_request` only.

Source-manifest generation and verification passed after the documentation and evidence refresh. The final
handoff check reported a clean Git worktree after both commits and archive creation.

No mainnet action was performed. No canister was deployed, installed, upgraded, funded, reconfigured, aliased,
or assigned different controllers. Production `SURPLUS_CANISTER` remains `None`.
