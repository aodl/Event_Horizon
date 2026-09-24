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

The validated encodings at IDs 0–2 are unchanged. Upgrading from `c48778d1744f3b645d0165cfdccec2ee704db71c` initializes only the new structures. It neither fabricates pricing history nor materializes global subscriptions from old account records.

Account `minimum_e8s = 0` remains the unambiguous omitted-threshold sentinel. Global declarations have no synthetic account or subaccount record.

Durable state includes the Ledger cursor, permanent admissions, cadence mode, deterministic CMC identity, bounded pricing history, and epoch decisions. Poll match accumulators, timer IDs, single-flight leases, decoded pages, and logging suppression remain transient.
