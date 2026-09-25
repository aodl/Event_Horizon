# Frontend

The separately controlled Rust frontend embeds and certifies the supplied static assets. Its builder offers Global Ledger, Single subaccount, and Subaccount range modes, preserves the exact declaration text, continuously shows the complete ASCII byte count, and enforces the 32-byte Jupiter Faucet memo limit. Range fields are start, end, and optional minimum ICP; validation never reverses endpoints, drops thresholds, shortens values, or creates multiple memos.

The embedded browser bundle uses `@icp-sdk/core` to invoke the permanent backend `eo6ei-gaaaa-aaaar-qchra-cai` through `https://icp-api.io`. The principal and API host are explicit constants in the certified JavaScript asset and are consequently committed into the frontend Wasm. The production path does not use runtime discovery or fetch a root key; the mainnet root key is embedded by the agent. The frontend canister has no HTTP update endpoint or pricing proxy.

The page displays current account/range/global prices, current and next UTC boundaries, freeze time, all three frozen prices, stale carry-forward state, and floor/latest observations. It explains that range width does not affect the 20-ICP economic basis. CMC integers are formatted for display only with four decimal places: `23147` is shown as `2.3147 XDR/ICP`. The wording identifies these values as Event Horizon's recorded CMC observations, not market lows.

Static HTML, JavaScript, CSS, and SVG bytes remain embedded and certified. The pricing value shown is protocol authoritative because it comes from the backend query. Backend admission remains authoritative if the display is stale, unavailable, or modified by a frontend controller.

When a frozen next price is higher, the builder recommends that amount immediately and explains Faucet delivery delay. When it is lower, the builder continues recommending the current amount until activation. Backend evaluation remains authoritative.

Static assets retain certified query responses. Pricing is obtained at runtime from the read-only backend query rather than inserted into the frontend's static certificate tree. No production backend method beyond `get_pricing` is used.
