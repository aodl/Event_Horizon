# Event Horizon frontend canister

The frontend is a separately controlled informational Rust canister. It embeds and certifies static HTML, JavaScript, CSS, and SVG assets; those exact asset bytes become part of the frontend Wasm and retain certified query responses.

HTML retains the non-stale `public, no-cache, no-store` policy. Mutable JavaScript, CSS, and SVG responses use `public, no-cache`, so browsers may store them but must revalidate before reuse. The HTML references `app.bundle.js?v=2` and `styles.css?v=2` once to bypass clients that may have cached the former bare URLs under the historical one-year immutable policy. This fixed suffix is a transition, not an asset-versioning pipeline; subsequent changes rely on revalidation. The certified router serves the query-suffixed URLs without weakening response certification or CSP.

## Static instance and network configuration

The committed static instance registry identifies each reviewed backend, observed ledger, optional SNS Root, optional surplus recipient, alias, and expected backend Wasm hash. The current browser bundle uses `@icp-sdk/core` to call the reviewed backend through `https://icp-api.io`. The API host and canister principals are explicit certified JavaScript constants.

The production agent uses the embedded mainnet root key. It performs no runtime canister discovery and never fetches a root key. Before accepting runtime pricing, `get_instance` verifies the backend's reported observed Ledger, SNS Root, surplus recipient, and discovered profile against the static registry. The information panel displays the reviewed recipient or `none`. `get_pricing` then supplies authoritative current and frozen admission pricing. A monotonically increasing selection generation binds each response to the captured registry entry; a late success or failure from an earlier selection cannot change verification state, pricing, information, or builder controls.

This browser comparison verifies reported configuration only. Installed module hash and controller state remain separate operator/reviewer checks; the frontend makes no runtime management-canister call.

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
