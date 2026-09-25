# Production surface

The backend exposes exactly two bounded queries:

```candid
get_instance : () -> (InstanceInfo) query;
get_pricing : () -> (Pricing) query
```

They expose immutable configuration/profile and pricing, never subscriptions or administration. The audit requires exactly `canister_query get_instance` and `canister_query get_pricing` and rejects all other methods. Debug inspection remains behind `debug_api`.

Each current/frozen public price contains `account_icp`, derived `range_icp`, and `global_icp`. This record extension adds no method and does not alter the export surface.

Native public canister status and logs remain the operational interface. Verify the production Candid/export surface, trust anchors, public visibility settings, and reproducible Wasm hash before controller removal.
