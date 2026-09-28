# Frontend

The separately controlled Rust frontend embeds and certifies the supplied static assets. Its builder offers Global Ledger, Single subaccount, Subaccount range, Single neuron nonce, and Neuron nonce range modes. Target parsing uses `BigInt` across the full `u64` domain; ranges contain at most 256 targets. Neuron modes require live `neuron_governance`. The builder preserves exact declaration text, enforces the complete 32-byte Jupiter memo limit, and verifies both observed Ledger and optional SNS Root against the static registry.

The embedded browser bundle uses `@icp-sdk/core` to invoke the permanent backend `eo6ei-gaaaa-aaaar-qchra-cai` through `https://icp-api.io`. The principal and API host are explicit constants in the certified JavaScript asset and are consequently committed into the frontend Wasm. The production path does not use runtime discovery or fetch a root key; the mainnet root key is embedded by the agent. The frontend canister has no HTTP update endpoint or pricing proxy.

The page displays current account/range/global prices, current and next UTC boundaries, freeze time, all three frozen prices, stale carry-forward state, and floor/latest observations. It explains that range width does not affect the 20-ICP economic basis. CMC integers are formatted for display only with four decimal places: `23147` is shown as `2.3147 XDR/ICP`. The wording identifies these values as Event Horizon's recorded CMC observations, not market lows.

Static HTML, JavaScript, CSS, and SVG bytes remain embedded and certified. The pricing value shown is protocol authoritative because it comes from the backend query. Backend admission remains authoritative if the display is stale, unavailable, or modified by a frontend controller.

When a frozen next price is higher, the builder recommends that amount immediately and explains Faucet delivery delay. When it is lower, the builder continues recommending the current amount until activation. Backend evaluation remains authoritative.

Static assets retain certified query responses. Runtime values come from exactly the backend's two production queries: `get_instance` verifies the reviewed immutable instance configuration/profile, and `get_pricing` supplies authoritative admission pricing. Neither query is inserted into the frontend's static certificate tree, and the frontend has no update or administrative access to backend state.
