# Production surface

The backend exposes exactly one application method:

```candid
get_pricing : () -> (Pricing) query
```

It is read-only, tightly bounded, and contains no subscription, status, log, recovery, or administrative data. The release export audit requires exactly `canister_query get_pricing` and rejects every other application query, composite query, or update. Debug inspection remains behind `debug_api` and must be absent from canonical production Wasm.

Native public canister status and logs remain the operational interface. Verify the production Candid/export surface, trust anchors, public visibility settings, and reproducible Wasm hash before controller removal.
