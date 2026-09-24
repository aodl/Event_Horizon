# Trust model

Event Horizon is a latency aid. Subscriber reconciliation is authoritative.

- The ICP Ledger defines processed activity and fixed poll boundaries.
- Jupiter Faucet origin plus a complete Jupiter Historian exact-route total define admission.
- The CMC's ICP/XDR conversion-rate query is the only pricing oracle.
- `get_pricing` is the backend's only production application method and exposes no subscription data.
- Range declarations are protocol-bounded to 256 admission-time watched-account merges; they add no per-transfer scan, per-range scheduler, or additional poke allowance.
- Public native status and logs remain the operational interface.
- The mutable frontend presents pricing obtained by a direct read-only backend query but cannot change backend admission decisions. Its embedded assets are certified; backend admission remains authoritative.

Daily samples can be missed, and the observed rolling minimum is only the lowest successful daily observation recorded by Event Horizon. It is not a market low. Stale data carries prices forward. Pokes can fail, be delayed, or arrive before Index visibility.

Controller removal remains an irreversible action after a controlled observation period. The backend contains no administrative, recovery, install-code, or self-upgrade method.
