# Trust model

Event Horizon is a latency aid. Subscriber reconciliation is authoritative.

- The ICP Ledger defines processed activity and fixed poll boundaries.
- Jupiter Faucet origin plus a complete Jupiter Historian exact-route total define admission.
- The CMC's ICP/XDR conversion-rate query is the only pricing oracle.
- Liquid cycles observed hourly are the only adaptive-surplus health signal.
- `get_pricing` is the backend's only production application method and exposes no subscription data.
- Range declarations are protocol-bounded to 256 admission-time watched-account merges; they add no per-transfer scan, per-range scheduler, or additional poke allowance.
- Public native status and logs remain the operational interface.
- The mutable frontend presents pricing obtained by a direct read-only backend query but cannot change backend admission decisions. Its embedded assets are certified; backend admission remains authoritative.

Daily samples can be missed, and the observed rolling minimum is only the lowest successful daily observation recorded by Event Horizon. It is not a market low. Stale data carries prices forward. Pokes can fail, be delayed, or arrive before Index visibility.

Surplus epochs likewise use an observed hourly minimum, not continuous monitoring or a burn forecast. The current-balance gate and retained-first ordering protect service even when the stored level is high. The immutable receiver is trusted only as the destination account owner: Event Horizon calls no receiver endpoint, grants it no control, and makes no claim about how it governs or spends received ICP. Direct donations and Faucet funding share the same pool; funding source never changes subscriber priority.

Controller removal remains an irreversible action after a controlled observation period. Before it, the currently `None` production surplus constant must either remain deliberately disabled or be changed to one reviewed immutable principal and rebuilt. The backend contains no administrative, destination, withdrawal, recovery, install-code, or self-upgrade method.
