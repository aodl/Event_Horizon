# Production surface

The canonical backend contains both fixed protocol adapters. Equality with the compiled canonical ICP principal selects the legacy `query_blocks` reader for a shared admission/observation scan; every other observed principal uses ICRC-3 while admission remains on the canonical legacy ICP reader. Reader selection is not an application method and cannot be changed after installation.

The backend exposes exactly three bounded queries:

```candid
get_instance : () -> (InstanceInfo) query;
get_pricing : () -> (Pricing) query;
preview_subscription : (text) -> (variant { Ok : SubscriptionPreview; Err : text }) query;
```

They expose immutable configuration/profile, pricing, and a bounded local-state-only declaration preview. Preview returns resolved subscriber/subject, scope, endpoints/count, optional raw threshold, and cached alias revision/refresh time, or an error. It performs no awaits, admission, funding verification, writes or pricing mutation. The audit requires exactly `canister_query get_instance`, `canister_query get_pricing`, and `canister_query preview_subscription` and rejects all other application methods. Debug inspection remains behind `debug_api`.

Each current/frozen public price contains `account_icp`, derived `range_icp`, and `global_icp`. This record extension adds no method and does not alter the export surface.

Native public canister status and logs remain the operational interface. Verify the production Candid/export surface, trust anchors, public visibility settings, and reproducible Wasm hash before controller removal.
