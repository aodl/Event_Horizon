# Production surface

`canisters/event-horizon/event_horizon.did` is intentionally:

```candid
service : () -> {}
```

Debug inspection belongs behind the `debug_api` build feature and must never be present in the canonical production Wasm. Production observability uses native public canister status and public logs.

Before eventual controller removal, verify the production Candid/export surface, compiled trust anchors, public visibility settings and reproducible Wasm hash.
