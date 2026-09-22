# Frontend

The separately controlled Rust frontend embeds and certifies the supplied static assets. Its builder preserves the exact declaration text and enforces the 32-byte Jupiter Faucet memo limit.

The browser fetches `/pricing.json`. The frontend canister upgrades that path to an HTTP update, reads the ICP CLI injected `PUBLIC_CANISTER_ID:event_horizon`, and calls `get_pricing`. It displays current account/global prices, current and next UTC boundaries, freeze time, frozen prices, stale carry-forward state, and floor/latest observations.

When a frozen next price is higher, the builder recommends that amount immediately and explains Faucet delivery delay. When it is lower, the builder continues recommending the current amount until activation. Backend evaluation remains authoritative.

Static assets retain certified query responses. The pricing response is produced by a consensus update rather than inserted into the static certificate tree. No production backend method beyond `get_pricing` is used.
