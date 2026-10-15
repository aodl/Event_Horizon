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

## Landing ledger total

The hero's **ICP ledger transactions** total is the entire ICP ledger history,
including activity before Event Horizon and transaction kinds beyond transfers.
A future reviewed ICRC-3 entry uses **<registry name> ledger records**. This does
not measure Event Horizon processing, cursor lag, notifications, current backend
polling or health. A backend cursor starts prospectively and may skip explicitly
proved archive intervals; it is not a true processing count.

`ledger-counter-client.js` uses the selected registry entry's observed ledger
and explicit `ledgerCountSource`. ICP opts into `icp-chain-length`; IO remains
planned with `null` and no request. To add a reviewed live generic entry, verify
its principal and zero-length ICRC-3 interface, set `icrc3-log-length`, and retain
records wording unless its transaction semantics are separately reviewed. Never
infer this mode from a token symbol, alias, SNS association or endpoint probe.

The only requests are `query_blocks({start:0n,length:0n})` with `nat64` fields,
or `icrc3_get_blocks([{start:0n,length:0n}])` with `nat` fields. The returned
`chain_length` / `log_length` is already a count. Response width projection is
wire-tested against independent full schemas, including recursive values and
archive function references; no callback is executed or body requested.
Unsupported calls and malformed or decreasing totals become component failures.

| Setting | Default |
| --- | --- |
| First sample | Immediately |
| One warm-up | 3 seconds after the first accepted baseline |
| Steady sample | 15 seconds after the previous attempt finishes |
| Linear interpolation | 15 seconds |
| Logical attempt deadline, including initialization | 10 seconds |
| Failure retry | 30, 60, then at most 120 seconds |
| Delayed indication | 45 seconds without an accepted sample |
| Inconsistent indication | Three lower responses since acceptance |

`ledger-counter.js` separates accepted total, displayed integer, target,
monotonic sample time, wall-clock receipt timestamp and status. BigInt arithmetic
preserves even generic totals above `u64::MAX`. The first sample is direct,
equal samples do not extend smoothing, and no value is extrapolated. Once the
latest target is reached there are no numeric animation frames. Digit grouping
preserves every integer; long values wrap rather than overflow or truncate.

The visible explanation describes smoothing lag and that **Last checked** means
accepted-response time, not proof of freshest replica state. Failures retain the
last known number and timestamp; stale status has its own timer. A lower sample
never silently becomes an accepted high-water value. A later nondecreasing reply
can recover. After a delayed episode or suspension the next sample is direct.

Hidden pages suspend numeric updates and polling and pause both CSS animation
and the artwork's SVG timeline. Visible but unfocused pages keep ledger sampling
while artwork pauses. Resuming coalesces events into one refresh. Dynamic reduced
motion shows direct samples and static artwork. There is no manual pause button
or manual-pause state. Repeated object loads honor automatic suspension. The
visual numeral is hidden from the accessibility tree; a quiet text alternative
exposes the latest exact sampled total. No samples or totals are persisted.

The client lazily reuses an agent and actor, uses the embedded mainnet root key
and default verification, and sets `retryTimes: 0`. Its abort-aware fetch covers
query and signature-key requests, including initialization. On retirement or
failure the session is permanently aborted and evicted; it cannot borrow the
next attempt's signal. Pricing has separate ownership. The DOM integration starts
without waiting for pricing, and only a typed positive configuration mismatch
retires the statistic. Failure or absence of the optional component leaves the
builder operational, still requiring successful configuration verification.

Certification covers the static bundle and markup, **not** a separately queried
runtime ledger total. No proof-backed tip-certificate feature is claimed. This
release changes only frontend assets and tests; backend settings, state, Candid,
principals and reviewed backend hash stay unchanged.

For deterministic tests, real-browser checks, opt-in read-only live smoke and
limitations, see [testing](../../docs/development/testing.md) and the
[ledger-counter validation report](../../docs/codex/ledger-counter-validation.md).

## Hero branding and sampled recent rate

The compact hero restores the art-first spacing used before the counter
(`41e088f`, following the layout in `63c4022`): the full heading, lead, CTA and
facts follow the cropped SVG's complete height. Only the ledger statistic stays
in its own lower-right artwork region, capped to the first viewport when the
art is taller. Its label, left-hand unbracketed rate and right-aligned count stay
unboxed; long/error content expands the row rather than overlapping the body.
Wide desktop keeps its title/artwork composition and proximity to fact 04.

The `EVH.` shortcut fits the dark corner in compact landscape while the large
heading would obscure the artwork. Show it only if the full heading is actually
wholly below the viewport; hide it on any partial heading visibility or when
it leaves upward. Tall screens, scrolling, text zoom and reflow make the media
condition alone insufficient. The link defaults hidden, uses viewport observation
and an initial geometry check, and fails closed without observation. Cleanup
removes observers/listeners/frames. Keep the comments and browser regression
checks when changing hero spacing; never show both titles simultaneously.

Routine status and About copy stay visually hidden; the description remains
accessible. Errors remain visible. There is no manual pause control.

`n tx/s` is sampled recent-average growth in the same canonical ICP ledger's
historical transaction total, not backend processing, animation speed or
instantaneous network TPS. Generic ICRC-3 entries use `records/s`. The controller
retains up to 32 accepted receipt samples from the last 60 seconds and requires
at least 3 seconds between oldest and newest. Before the 3-second warm-up supplies a qualifying second sample, show
an unavailable-rate placeholder (`— tx/s` or `— records/s`). The placeholder also remains visible during delay and retry; planned, unsupported and configuration-mismatched selections omit it. Actual monotonic receipt spacing supplies the denominator; BigInt
fixed-point arithmetic preserves huge deltas. Rates round to one decimal; zero
is genuine and positive rates below 0.1 say `<0.1`. No extra request or timer is
introduced. Quiet accepted totals eventually give zero, not extrapolation.

Failures, lower replies, delayed data, selection/mismatch and
disposal clear the rate. Hiding clears only the sample history and holds the displayed estimate. Resume/recovery needs a fresh baseline and
qualifying span. Reduced motion still samples rates but displays totals directly.
The rate is not live-announced. Existing immediate/3-second warm-up/15-second
steady sampling, 10-second deadline, 30/60/120-second backoff and 45-second delayed
threshold remain unchanged. The screen-reader description explains smoothing lag and runtime
trust: static asset certification does not certify queried totals or rates.

Tab suspension retains the last displayed sampled rate while resetting its receipt-sample history. On return, hold that estimate until at least 3 seconds of new accepted samples can replace it; identify the held estimate as last known in its accessible description. Hidden tabs still suspend polling and artwork. Failures, contradictory samples, freshness expiry during active sampling, and selection changes still clear the rate.

The first rate may use the short three-second warm-up span and is correspondingly noisy; subsequent samples grow the averaging span toward the existing one-minute window. The three-second wait starts after the first accepted response; network latency can delay availability. Steady polling remains 15 seconds after attempt completion.
