# Stable-memory contract

Stable-memory IDs are permanent protocol identifiers and are never reused.

| ID | Meaning |
|---:|---|
| 0 | Metadata: admission/observed bootstrap flags and cursors, polling mode, health-log day |
| 1 | Watched-account subscription map |
| 2 | Immutable instance configuration and once-discovered observed profile |
| 3 | Debug-only runtime configuration |
| 4 | Global subscriber set keyed directly by principal |
| 5 | Pricing observations keyed by UTC day |
| 6 | Pricing initialization, current/frozen epochs, timestamps, attempt day, and measured call cost |
| 7 | `FundingState`: the complete authoritative retained/CMC/surplus operation |
| 8 | `SurplusPolicyState`: initialization, epoch start, hourly observed minimum, and level `0..19` |

No migration from the pre-generic acceptance deployment or intermediate development schemas is supported. The accepted transition is a deliberate reinstall because there are no subscribers.

ID 7 initializes directly to `FundingState::Idle`. Its only states are `Idle`, `CmcTransferPending`, `CmcNotifyPending`, and `SurplusTransferPending`. One cell always determines the next money-moving action, so no crash boundary exists between an authoritative split plan and its execution state.

Every new split freezes one `PlannedSurplus { destination, memo, amount_e8s, fee_e8s }` before the retained transfer begins. `None` denotes a CMC-only plan. The value survives `CmcTransferPending` to `CmcNotifyPending`; after successful minting and the second health gate, `SurplusTransferPending` adds the deterministic creation time to the complete identity. Recovery reads no destination or memo from current configuration.

ID 8 defaults to an uninitialized level-zero policy. This is live protocol state, not schema migration: destination-disabled operation accrues no entitlement, and enabling begins a fresh epoch. Levels above 19 or backwards-time state fail closed to a new level-zero epoch. Current-Wasm upgrades cover every pending funding phase and policy persistence.

Account `minimum_units = 0` remains the omitted-threshold sentinel; other values are arbitrary-precision observed-token units. Each admitted range expands into the ID 1 map. Its bounded key is protocol-tagged: canonical ICP stores `0x00` plus the 32-byte legacy AccountIdentifier; non-ICP stores `0x01`, one principal-length byte, principal bytes, and the effective 32-byte ICRC subaccount. Absent and explicit-zero ICRC subaccounts therefore normalize identically.

The stable pricing `Price` is `{ account_icp, global_icp }`. Public `get_pricing` values derive `range_icp = ceil(global_icp / 5)` with quotient/remainder arithmetic, exactly equal to `ceil(20F/C)`.

Durable state includes the Ledger cursor, permanent admissions, cadence mode, deterministic retained/surplus identities, the once-classified financial plan, bounded pricing history, and epoch decisions. Poll match accumulators, timer IDs, single-flight leases, decoded pages, and logging suppression remain transient. Debug liquid-balance overrides are test-only and transient.
