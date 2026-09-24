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
| 7 | FundingStateV2: the complete authoritative retained/CMC/surplus operation |
| 8 | Surplus policy initialization, epoch start, hourly observed minimum, and level `0..19` |

The validated encodings at every ID are unchanged by range support. The exact upgrade regression starts from `c3cb4b41f390cbe7ce81e8ef6976ff2527ee5161` with account/global subscriptions, Ledger cursor, CMC state, initialized pricing, and a frozen next price. Upgrade preserves them byte-compatibly and creates no phantom range entries.

Surplus support is additive. IDs 0–6 are never reinterpreted. ID 7 initializes as `Uninitialized`; its first read migrates ID 2 exactly once: old `Idle` becomes new `Idle`, old `TransferPending` becomes `CmcTransferPending` with both planned-surplus fields zero, and old `NotifyPending` becomes `CmcNotifyPending` with both planned-surplus fields zero. Thereafter only ID 7 is authoritative and old ID 2 remains untouched historical storage. One cell always determines the next money-moving action, so no crash boundary exists between an authoritative split plan and its execution state.

ID 8 defaults to an uninitialized level-zero policy. Destination-disabled observation resets that default and accrues no entitlement. Levels above 19 or backwards-time state fail closed to a new level-zero epoch. The exact `304b26e9e6facc6f8812e61c56f98e712e05f803` debug fixture covers upgrade of old Idle, TransferPending, and NotifyPending states; current-Wasm upgrades cover all three FundingStateV2 pending phases and policy persistence.

Account `minimum_e8s = 0` remains the unambiguous omitted-threshold sentinel. Each admitted range is expanded into the existing ID 1 map; no range registry or new memory ID exists. Overlapping declarations merge to the lowest threshold, which is safe because admissions are permanent and thresholds never become more restrictive. Global declarations have no synthetic account or subaccount record.

The stable pricing `Price` remains `{ account_icp, global_icp }`. Public `get_pricing` values derive `range_icp = ceil(global_icp / 5)` with quotient/remainder arithmetic, exactly equal to `ceil(20F/C)`. Frozen and current state therefore required no migration.

Durable state includes the Ledger cursor, permanent admissions, cadence mode, deterministic retained/surplus identities, the once-classified financial plan, bounded pricing history, and epoch decisions. Poll match accumulators, timer IDs, single-flight leases, decoded pages, and logging suppression remain transient. Debug liquid-balance overrides are test-only and transient.
