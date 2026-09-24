# Frontend

The separately controlled Rust frontend embeds and certifies the supplied static assets. Its builder preserves the exact declaration text and enforces the 32-byte Jupiter Faucet memo limit.

The embedded browser bundle uses `@icp-sdk/core` to invoke the backend's read-only `get_pricing` query directly. It discovers the backend from the ICP CLI injected `PUBLIC_CANISTER_ID:event_horizon` value carried in the standard `ic_env` cookie; no development canister ID is hardcoded. The frontend canister has no HTTP update endpoint or pricing proxy.

The page displays current account/global prices, current and next UTC boundaries, freeze time, frozen prices, stale carry-forward state, and floor/latest observations. CMC integers are formatted for display only with four decimal places: `23147` is shown as `2.3147 XDR/ICP`. The wording identifies these values as Event Horizon's recorded CMC observations, not market lows.

Static HTML, JavaScript, CSS, and SVG bytes remain embedded and certified. The pricing value shown is protocol authoritative because it comes from the backend query. Backend admission remains authoritative if the display is stale, unavailable, or modified by a frontend controller.

When a frozen next price is higher, the builder recommends that amount immediately and explains Faucet delivery delay. When it is lower, the builder continues recommending the current amount until activation. Backend evaluation remains authoritative.

Static assets retain certified query responses. The pricing response is produced by a consensus update rather than inserted into the static certificate tree. No production backend method beyond `get_pricing` is used.
