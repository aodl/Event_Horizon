# Event Horizon frontend canister

The frontend is a separately controlled informational Rust canister. It embeds and certifies static HTML, JavaScript, CSS, SVG, PNG, and ICO assets; those exact asset bytes become part of the frontend Wasm and retain certified query responses. The main document declares the bundled multi-size favicon. Unknown paths receive a certified HTML `404` response which uses the bundled 512-pixel Event Horizon icon and links back to the homepage.

HTML retains the non-stale `public, no-cache, no-store` policy. Mutable JavaScript, CSS, and image responses use `public, no-cache`, so browsers may store them but must revalidate before reuse. The current HTML references `app.bundle.js?v=3` and `styles.css?v=9`; query-string changes have historically bypassed URLs cached under the former one-year immutable policy, but they are not an asset-versioning pipeline. Ongoing changes rely on revalidation rather than requiring a new suffix. The certified router serves query-suffixed asset URLs without weakening response certification or CSP.

## Animated artwork

The hero uses the self-contained `event-horizon.svg`, which adds the approved warm ivory rim-glow animation beneath the original foreground texture patches. The 952,419-byte asset preserves the static master’s view box, title, description, 15 embedded images, 118 original paths, and 55 original IDs; its 10 motion-specific IDs are uniquely prefixed with `ehm-`. The 936,331-byte static master is retained as `event-horizon-static.svg` for the reduced-motion presentation. Both SVG roots use the same right-aligned `slice` crop as the placeholder so every hero aspect ratio is fully covered without letterboxing.

The production page declares the animated relative URL directly on the same-origin `<object>` and replaces it with the static asset when the page-level `prefers-reduced-motion` preference matches; the animated asset also keeps a best-effort internal guard for direct viewing. An image preload is deliberately not used because an object-document request cannot consume it. HTML responses retain `frame-ancestors 'none'`, while SVG responses use a dedicated policy that permits only same-origin embedding and the certified SVG's inline styles. The object integration lets the parent page pause both CSS and SVG timeline animation while the document is hidden or the browser loses focus, then resume from the preserved frame when it becomes active. Both variants retain the existing cover crop, right-aligned portrait crop, blurred placeholder, load-aware fade, and responsive layout.

Validation checks XML parsing, unique IDs, resolved fragment references, the 7 motion bundles and 18 glow uses, unchanged original image/path attributes, placement below all 14 foreground texture patches, absence of scripts and event handlers, the reduced-motion fallback, and the one-megabyte cap. Five secondary huge-blur strokes were removed to reduce animated filter work; the two primary outer halos remain, and the corresponding mid glows are slightly brighter. Firefox, Safari, and low-powered mobile performance remain browser/device review items.

## Static instance and network configuration

The committed static instance registry identifies each reviewed backend, observed ledger, optional SNS Root, optional surplus recipient, alias, and expected backend Wasm hash. The current browser bundle uses `@icp-sdk/core` to call the reviewed backend through `https://icp-api.io`. The API host and canister principals are explicit certified JavaScript constants.

The production agent uses the embedded mainnet root key. It performs no runtime canister discovery and never fetches a root key. Before accepting runtime pricing, `get_instance` verifies the backend's reported observed Ledger, SNS Root, surplus recipient, and discovered profile against the static registry. The information panel displays the reviewed recipient or `none`. `get_pricing` then supplies authoritative current and frozen admission pricing. Selecting another live instance synchronously replaces the previous memo with the placeholder, removes its funding instruction, and disables the builder until verification finishes. A monotonically increasing selection generation binds each response to the captured registry entry; a late success or failure from an earlier selection cannot change verification state, pricing, information, or builder controls.

This browser comparison verifies reported configuration only. Installed module hash and controller state remain separate operator/reviewer checks; the frontend makes no runtime management-canister call.

Production exposes only certified `http_request`. There is no HTTP update endpoint, pricing proxy, runtime registry mutation, root-key fetch, or administrative application API. Runtime query values are deliberately not inserted into the frontend's static certificate tree.

## Declaration builder

The browser builds all five declaration modes: global ledger, single subaccount, subaccount range, single neuron nonce, and neuron nonce range. Target parsing uses `BigInt` throughout the full `u64` domain; ranges contain at most 256 inclusive targets. Neuron modes are available only when the verified profile supplies `neuron_governance`.

The builder preserves exact declaration text and enforces Jupiter Faucet's complete 32-byte memo limit after Principal validation and normalization. Backend parsing, Historian route evidence, pricing, and admission remain authoritative even if the display is stale, unavailable, or modified by a frontend controller.

## Pricing display

The page displays the backend's authoritative current account/range/global
prices, current and next UTC boundaries, freeze time, all three frozen prices,
stale carry-forward state, and floor/latest CMC observations. It does not
calculate or hardcode admission prices in the browser. A freshly initialized
backend begins from the 10, 20, and 1,000 ICP reference bases, but independent
upward rounding means the displayed dynamic prices need not preserve an exact
ratio. Range width does not change the 20-ICP economic basis. CMC integers are
formatted for display with four decimal places (`23147` becomes `2.3147
XDR/ICP`) and are described as Event Horizon's retained CMC observations rather
than market lows.

When a frozen next price is higher, the builder recommends that amount
immediately to account for Faucet delivery delay. When it is lower, it
continues recommending the current amount until activation. The amount is the
minimum qualifying cumulative productive endowment: the principal remains in
Jupiter's staking path, while attributable raw-ICP payouts help fund Event
Horizon after conversion to cycles. Admission is permanent, but neither the
frontend nor that funding design guarantees a payout, cadence, yield, or
uninterrupted service. In every case, backend evaluation is authoritative.

## Development

```bash
cargo run -p xtask -- frontend_setup
npm run build:frontend
npm run test:frontend-unit
```

See the [architecture overview](../../docs/architecture/overview.md), [production surface](../../docs/architecture/production-surface.md), [deployment guide](../../docs/operations/deployment.md), and [subscriber guide](../../docs/subscriber-guide.md).
