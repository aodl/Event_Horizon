# Stable-memory contract

Stable-memory IDs are protocol storage identifiers and must never be reused for another meaning.

| Memory ID | Meaning |
|---:|---|
| `0` | Metadata cell: prospective-bootstrap flag, durable next Ledger block, current polling mode |
| `1` | Watched-account → effective subscription map |
| `2` | CMC conversion state cell |
| `3` | Debug-only runtime configuration; absent from the canonical production behaviour |

An admitted subscription stores a `minimum_e8s` value. `0` is reserved internally to mean the declaration omitted a threshold and every incoming transfer is relevant; explicit thresholds can never be below `0.01 ICP`, so this sentinel is unambiguous.

## Durable versus transient state

Durable state exists only where losing it would change future correctness or risk real funds:

- Ledger cursor;
- permanent admitted subscriptions;
- cadence hysteresis mode;
- deterministic CMC transfer/notify identity.

The following is intentionally transient:

- current poll's subscriber → matched-subaccounts map;
- timer IDs;
- in-flight poll/funding leases;
- decoded Ledger pages;
- logging suppression flags.

A pre-blackhole upgrade can therefore lose best-effort pokes from an incomplete poll without inventing delivery recovery machinery. Subscribers' own reconciliation remains authoritative.
