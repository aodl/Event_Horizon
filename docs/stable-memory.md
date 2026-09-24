# Stable-memory contract

Stable-memory IDs are permanent protocol identifiers and are never reused.

| ID | Meaning |
|---:|---|
| 0 | Existing metadata: bootstrap, Ledger cursor, polling mode |
| 1 | Existing watched-account subscription map |
| 2 | Existing CMC conversion state |
| 3 | Debug-only runtime configuration |
| 4 | Global subscriber set keyed directly by principal |
| 5 | Pricing observations keyed by UTC day |
| 6 | Pricing initialization, current/frozen epochs, timestamps, attempt day, and measured call cost |

The validated encodings at every ID are unchanged by range support. The exact upgrade regression starts from `c3cb4b41f390cbe7ce81e8ef6976ff2527ee5161` with account/global subscriptions, Ledger cursor, CMC state, initialized pricing, and a frozen next price. Upgrade preserves them byte-compatibly and creates no phantom range entries.

Account `minimum_e8s = 0` remains the unambiguous omitted-threshold sentinel. Each admitted range is expanded into the existing ID 1 map; no range registry or new memory ID exists. Overlapping declarations merge to the lowest threshold, which is safe because admissions are permanent and thresholds never become more restrictive. Global declarations have no synthetic account or subaccount record.

The stable pricing `Price` remains `{ account_icp, global_icp }`. Public `get_pricing` values derive `range_icp = ceil(global_icp / 5)` with quotient/remainder arithmetic, exactly equal to `ceil(20F/C)`. Frozen and current state therefore required no migration.

Durable state includes the Ledger cursor, permanent admissions, cadence mode, deterministic CMC identity, bounded pricing history, and epoch decisions. Poll match accumulators, timer IDs, single-flight leases, decoded pages, and logging suppression remain transient.
