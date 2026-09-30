# Event Horizon frontend canister

The frontend is a separately controlled informational Rust canister. It embeds and certifies static HTML, JavaScript, CSS, and SVG assets; those exact asset bytes become part of the frontend Wasm and retain certified query responses.

## Static instance and network configuration

The committed static instance registry identifies each reviewed backend, observed ledger, optional SNS Root, optional surplus recipient, alias, and expected backend Wasm hash. The current browser bundle uses `@icp-sdk/core` to call the reviewed backend through `https://icp-api.io`. The API host and canister principals are explicit certified JavaScript constants.

The production agent uses the embedded mainnet root key. It performs no runtime canister discovery and never fetches a root key. Before accepting runtime pricing, `get_instance` verifies the backend's observed Ledger, SNS Root, surplus recipient, and discovered profile against the static registry. The information panel displays the reviewed recipient or `none`. `get_pricing` then supplies authoritative current and frozen admission pricing.

Production exposes only certified `http_request`. There is no HTTP update endpoint, pricing proxy, runtime registry mutation, root-key fetch, or administrative application API. Runtime query values are deliberately not inserted into the frontend's static certificate tree.

## Declaration builder

The browser builds all five declaration modes: global ledger, single subaccount, subaccount range, single neuron nonce, and neuron nonce range. Target parsing uses `BigInt` throughout the full `u64` domain; ranges contain at most 256 inclusive targets. Neuron modes are available only when the verified profile supplies `neuron_governance`.

The builder preserves exact declaration text and enforces Jupiter Faucet's complete 32-byte memo limit after Principal validation and normalization. Backend parsing, Historian route evidence, pricing, and admission remain authoritative even if the display is stale, unavailable, or modified by a frontend controller.

## Pricing display

The page displays current account/range/global prices, current and next UTC boundaries, freeze time, all three frozen prices, stale carry-forward state, and floor/latest CMC observations. Range width does not change the 20-ICP economic basis. CMC integers are formatted for display with four decimal places (`23147` becomes `2.3147 XDR/ICP`) and are described as Event Horizon's retained CMC observations rather than market lows.

When a frozen next price is higher, the builder recommends that amount immediately to account for Faucet delivery delay. When it is lower, it continues recommending the current amount until activation. In every case, backend evaluation is authoritative.

## Development

```bash
cargo run -p xtask -- frontend_setup
npm run build:frontend
npm run test:frontend-unit
```

See the [architecture overview](../../docs/architecture/overview.md), [production surface](../../docs/architecture/production-surface.md), [deployment guide](../../docs/operations/deployment.md), and [subscriber guide](../../docs/subscriber-guide.md).
