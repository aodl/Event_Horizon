# Event Horizon frontend canister

The frontend is a separately controlled informational Rust canister. It embeds and certifies static HTML, JavaScript, CSS, SVG, PNG, and ICO assets; those exact asset bytes become part of the frontend Wasm and retain certified query responses. The main document declares the bundled multi-size favicon. Unknown paths receive a certified HTML `404` response which uses the bundled 512-pixel Event Horizon icon and links back to the homepage.

HTML retains the non-stale `public, no-cache, no-store` policy. Mutable JavaScript, CSS, and image responses use `public, no-cache`, so browsers may store them but must revalidate before reuse. The current HTML references `app.bundle.js?v=3` and `styles.css?v=10`; query-string changes have historically bypassed URLs cached under the former one-year immutable policy, but they are not an asset-versioning pipeline. Ongoing changes rely on revalidation rather than requiring a new suffix. The certified router serves query-suffixed asset URLs without weakening response certification or CSP.

## Mainnet identity

- CLI name: `event_horizon_frontend`.
- Frontend canister: `ej7c4-lyaaa-aaaar-qchrq-cai`.
- Public origin: [www.event-horizon-icp.com](https://www.event-horizon-icp.com/).
- Canister URL: [ej7c4-lyaaa-aaaar-qchrq-cai.icp.net](https://ej7c4-lyaaa-aaaar-qchrq-cai.icp.net/).
- Canonical ICP backend: `event_horizon`, `eo6ei-gaaaa-aaaar-qchra-cai`;
  see its [README](../event-horizon/README.md).

From the repository root, check the existing frontend's mainnet mapping without
deploying:

```bash
icp canister status event_horizon_frontend -e ic --id-only
```

The expected result is `ej7c4-lyaaa-aaaar-qchrq-cai`. `icp.yaml` declares the
name, while the ignored operator-local `.icp/data/mappings/ic.ids.json` resolves
it for `-e ic`. Restore that mapping on a fresh checkout using the
[deployment runbook](../../docs/operations/deployment.md#canister-name-resolution).
The browser's reviewed backend registry is separate from CLI deployment mappings;
editing either does not update the other. Frontend upgrades target only the
frontend canister and do not require a backend upgrade.

## Custom origin and social preview

The canonical public origin is `https://www.event-horizon-icp.com/`. The
frontend embeds and certifies `/.well-known/ic-domains`, whose sole hostname is
`www.event-horizon-icp.com`, so ownership can be checked through the original
`ej7c4-lyaaa-aaaar-qchrq-cai.icp.net` hostname before registration. The apex
`event-horizon-icp.com` is outside the canister and is intended to be forwarded
permanently to `www` by GoDaddy.

The versioned `/og/event-horizon-preview-v1.jpg` is a locally hosted, certified
1200×630 sRGB JPEG. Canonical, Open Graph and X metadata are static in
`index.html`; the transaction number visible in that approved image is a fixed
snapshot, not a runtime Ledger reading. The image uses the existing revalidation
cache policy, while the ownership file uses `no-cache, no-store` so registration
checks cannot retain stale content. Both are public and require no JavaScript,
cookies, authentication, backend pricing or application initialization.

The pinned asset router certifies complete `GET` responses. It does not provide
a separate certified empty-body `HEAD` representation, so this release relies on
the fully tested `GET` path rather than adding a second HTTP-serving framework.
The browser application still sends ICP API requests to the reviewed
`https://icp-api.io` host; neither the custom page origin nor `window.location`
selects a replica gateway. Registration, DNS changes and frontend deployment are
separate operator actions documented in the
[deployment runbook](../../docs/operations/deployment.md).

## Animated artwork

The hero uses the self-contained `event-horizon.svg`, which adds the approved warm ivory rim-glow animation beneath the original foreground texture patches. The 952,419-byte asset preserves the static master’s view box, title, description, 15 embedded images, 118 original paths, and 55 original IDs; its 10 motion-specific IDs are uniquely prefixed with `ehm-`. The 936,331-byte static master is retained as `event-horizon-static.svg` for the reduced-motion presentation. Both SVG roots use the same right-aligned `slice` crop as the placeholder so every hero aspect ratio is fully covered without letterboxing.

The production page declares the animated relative URL directly on the same-origin `<object>` and replaces it with the static asset when the page-level `prefers-reduced-motion` preference matches; the animated asset also keeps a best-effort internal guard for direct viewing. An image preload is deliberately not used because an object-document request cannot consume it. HTML responses retain `frame-ancestors 'none'`, while SVG responses use a dedicated policy that permits only same-origin embedding and the certified SVG's inline styles. The object integration lets the parent page pause both CSS and SVG timeline animation while the document is hidden or the browser loses focus, then resume from the preserved frame when it becomes active. Both variants retain the existing cover crop, right-aligned portrait crop, blurred placeholder, load-aware fade, and responsive layout.

Validation checks XML parsing, unique IDs, resolved fragment references, the 7 motion bundles and 18 glow uses, unchanged original image/path attributes, placement below all 14 foreground texture patches, absence of scripts and event handlers, the reduced-motion fallback, and the one-megabyte cap. Five secondary huge-blur strokes were removed to reduce animated filter work; the two primary outer halos remain, and the corresponding mid glows are slightly brighter. Firefox, Safari, and low-powered mobile performance remain browser/device review items.

## Hero fact links and explanations

The four hero facts are local fragment links to `#open-source`, `#immutability`,
`#endowment-funding` and `#internet-computer`. Their four explanation subsections live inside The protocol, with level-three
headings and the original fragment IDs. Native anchors preserve URL/history,
direct links and keyboard target focus; the existing reduced-motion scroll rule
also applies. Icons have empty alternatives so each link has one complete text
name. Wide desktop retains four columns, tablet uses two and narrow mobile one.

The GitHub white mark, Jupiter Faucet ICO and ICP full-colour infinity SVG are
small local certified assets; Event Horizon reuses `/favicon.ico?v=1`.
No brand artwork is recoloured or distorted. The official ICP SVG's transparent
favicon clear space is retained; its 44px image box displays the wide infinity
at about 39×20px. The other icon boxes are 24px. No serving code, registry,
runtime client or production dependency changes are needed. Asset provenance,
hashes and validation evidence are in the
[hero-facts validation report](../../docs/codex/hero-facts-validation.md).

## Static instance and network configuration

The committed static instance registry identifies each reviewed backend, observed ledger, optional SNS Root, optional surplus recipient, alias, and expected backend Wasm hash. The current browser bundle uses `@icp-sdk/core` to call the reviewed backend through `https://icp-api.io`. The API host and canister principals are explicit certified JavaScript constants.

The production agent uses the embedded mainnet root key. It performs no runtime canister discovery and never fetches a root key. Before accepting runtime pricing, `get_instance` verifies the backend's reported observed Ledger, SNS Root, surplus recipient, and discovered profile against the static registry. The information panel displays the reviewed recipient or `none`. `get_pricing` then supplies authoritative current and frozen admission pricing. Selecting another live instance synchronously replaces the previous memo with the placeholder, removes its funding instruction, and disables the builder until verification finishes. A monotonically increasing selection generation binds each response to the captured registry entry; a late success or failure from an earlier selection cannot change verification state, pricing, information, or builder controls.

This browser comparison verifies reported configuration only. Installed module hash and controller state remain separate operator/reviewer checks. The operating-status panel separately uses read-only native management queries for public cycles and existing log reports; it does not perform management updates.

Production exposes only certified `http_request`. There is no HTTP update endpoint, pricing proxy, runtime registry mutation, root-key fetch, or administrative application API. Runtime query values are deliberately not inserted into the frontend's static certificate tree.

## Instance operating status

The Network panel shows the selected instance's current queried **total cycles
balance**, its **last reported polling tier**, and progress toward the next tier's
liquid-cycle entry threshold. `T` means one trillion cycles; displayed balances
are truncated to three decimals, while deficits/progress use BigInt arithmetic.
The reported liquid balance and its UTC timestamp are shown separately. Total
cycles are not interchangeable with immediately liquid cycles. Hysteresis also
means a balance alone cannot identify the operating tier.

`operating-status-client.js` reuses a separate anonymous pinned-SDK agent and
management actor, with normal signature verification and the embedded mainnet
root key. It invokes native **query** methods `canister_status` and
`fetch_canister_logs` with `{canister_id: selected.backendCanisterId}`, using that
same principal for effective routing. It does not call management updates, read
private operator credentials, change visibility, query another instance, or add
a backend endpoint. The existing deployment already publishes status and logs;
other instances lacking these permissions show unavailable.

The mode/liquid snapshot comes from the newest retained `HEALTH` (matching the
reviewed observed Ledger) or `POLL_MODE_CHANGE` record, ordered by log index.
It can be older than the current total balance. Missing reports or incompatible
log formats fail closed. Never use the older liquid reading as an exact current
top-up requirement. Entry thresholds remain Economy 2 T, Standard 5 T, Fast
10 T, Very fast 25 T and Continuous 100 T. The next threshold depends on the
reported mode, including Reserve Protection's 2 T re-entry to Economy. At the
highest tier there is no next threshold; a reached threshold does not itself
claim that the scheduler has changed mode.

Sampling is immediate, then 60 seconds after completion, with a 10-second
abortable deadline and zero SDK transport retries. Hidden pages abort/suspend
work; resume makes one refresh. Planned selection, disposal and positive
configuration mismatch retire requests and timers. Failures hide the readings
and retry independently, leaving the memo builder, pricing and hero ledger
counter unaffected. No animation or persistence is involved. These runtime
queries are not covered by static asset certification and do not establish
polling completion or notification delivery.

See the [operating-status validation](../../docs/codex/operating-status-validation.md)
and the ICP [management query documentation](https://docs.internetcomputer.org/references/management-canister/).

## Declaration builder

Step 02 contains the subscription inputs and a native link to step 03. Step 03
shows the single live generated declaration, validation and funding requirement
beside the Jupiter Faucet completion guidance. Moving between steps does not
submit a transaction; output still clears immediately on instance changes.
The completion area reuses the ledger-selection panel styles. Input errors name
02 / Compose, with a native link back to edit; verification and pricing states
remain separate from input corrections.

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

## Floating Fully on-chain map

One fixed left-edge native dialog has exactly three presentations. Fresh pages
start **Minimised**, one clickable 46px vertical tab. Its native `open` property avoids
autofocus, so the first Tab still reaches the page's Skip link. There is no
permanent dismissal, storage preference or automatic Dashboard startup.

| Presentation | Visible controls | Map interaction |
| --- | --- | --- |
| Minimised | Entire tab → Thumbnail | Hidden and inert |
| Thumbnail | Minimise (dash), Maximise (square) | Scaled preview; inert |
| Expanded | Minimise (dash), Restore thumbnail (stacked squares) | Interactive native modal |

Button names explicitly identify their destination. The thumbnail surface also
activates its single Maximise control; it adds no third button or keyboard stop.
Minimised and Thumbnail transitions update the existing open non-modal dialog.
Only modal/non-modal switching closes and reopens it. The same iframe remains
mounted and its URL is never reassigned during presentation changes. Escape
received by the parent returns to the preceding non-modal mode; direct
`#subnet-map` entry defaults Escape to Thumbnail. Escape inside the cross-origin
frame is not guaranteed to reach the parent, so toolbar controls stay visible.
Ordinary widget controls leave URL/history unchanged, including an existing
`#subnet-map`; a fresh load or history navigation to that fragment opens Expanded.
Accordion fragments and form state remain independent.

Both non-modal backgrounds meet `left: 0`. Left safe-area padding protects their
controls internally without a visible gutter; bottom placement respects the
bottom safe-area inset. Both non-modal modes use a 50px bottom offset (38px higher than before), falling back to 12px at viewport heights of 320px or less; Expanded remains centred. The vertical tab uses `writing-mode: vertical-rl` and a
180-degree title rotation. The entire tab is one native button with a visible focus ring; Enter/Space restore Thumbnail. Thumbnail geometry remains
600×520 CSS pixels uniformly scaled to .25, reserving 150×130 pixels inside a
224px-wide toolbar/frame. Its wrapper is inert and aria-hidden; iframe tabindex
is -1 and pointer events are disabled. The hidden minimised preview uses
`display: none` without removing or blanking the frame.

Exceptionally short or zoomed viewports bound the floating panel to the usable
height and allow scrolling without reducing its 44px control targets. Short
landscape layouts reserve space beside the panel for the compact EVH shortcut. Narrow
modal widths use the available layout width, including scrollbar space.

Expanded desktop width is `min(66.667vw, 1200px)`; below 900px it uses the viewport
minus modest safe-area margins. The modal stays centred, scrolls rather than
cropping the country list, and keeps its toolbar sticky. `subnet-layout.js`
retains one ResizeObserver: W ≤ 760 uses native responsive width and 520px height;
W > 760 scales a 760×520 layout uniformly by W/760 and reserves its full height.
Both non-modal presentations bypass that calculation. Missing observers leave
the expanded map at native responsive width. There is no scroll listener.

Widget headings are uppercase, weight 500 with consistent tracking and 14px left toolbar padding. Normal loading shows a centred 18px thumbnail / 1.75rem expanded CSS spinner with visually hidden status text; reduced motion uses a static ring. The existing 500ms settle, 12s slow explanation and late recovery are unchanged.

The loading overlay is a sibling of the inert frame wrapper, so its visually hidden status remains accessible in Thumbnail as well as Expanded. It is positioned over the same map region without changing the iframe or loading controller.

At effective viewport widths of 200px or less, the expanded toolbar gives its title a separate row, preserving both 44px controls without overlap. Actual Chromium zoom validation covers all eight requested window sizes at 100%, 150% and 200%.

The exact approved external URL remains:

```text
https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae
```

The first tab activation initializes the loader and assigns the URL exactly
once, after the requested presentation is visible. A direct `#subnet-map` counts
as an explicit request. Before activation there is no external src, Dashboard
request, Dashboard script execution or Dashboard loading timer. The initial blank
iframe document is not a loading milestone. Initial load checks inspect only the
parent-owned src attribute; no cross-origin Document getter or Dashboard DOM is
read. No-JavaScript visitors retain a conventional external link.

The unscaled dark loading layer clears 500ms after external document load, a
visual milestone rather than proof of data readiness. A 12-second deadline keeps
the placeholder and exposes a slow-loading explanation beside the permanent
fallback. Late load recovers, and minimisation/restoration does not reset the
loading state. Non-persisted page exit disposes timers and the observer; BFCache
restoration keeps the existing document, valid mode and frame. A full fresh load
starts Minimised again.

After first activation the complete independently maintained Dashboard application
runs, with its full JavaScript and data-request cost. Scaling saves no download
work. Hiding the preview can reduce rendering, but does not guarantee suspension
of external scripts, timers or network requests. No suspension protocol, resource
manager, runtime telemetry, external data integration or Event Horizon backend
query is added. Hero artwork, ledger polling/rates, status refresh, pricing and
preview safeguards are unchanged. Baseline/candidate traces and network figures
are recorded in the [original performance report](../../docs/codex/navigation-widget-validation.md).

The independent [standalone example](../../examples/subnet-map-widget/README.md)
uses only its own HTML/JS/CSS. It is neither imported into this bundle nor served
by the frontend canister. Its licensing decision is pending; no new licence is
granted by that example or this release.

The iframe retains `referrerpolicy="no-referrer"` and sandbox
`allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox`.
The certified page CSP retains `frame-src 'self' https://dashboard.internetcomputer.org`
and `frame-ancestors 'none'`; SVG policy is unchanged. The dashboard is independently
maintained and outside Event Horizon certification. Subnet composition proves
neither controller removal nor deployed source. Review the URL after subnet moves.
The dashboard full-page link must retain explicit `noopener noreferrer` and open a
working separate tab with null opener. Popup permissions do not restrict future
external popup destinations or enforce opener isolation on arbitrary new code.
No top navigation, external DOM manipulation or message protocol is introduced.

## Landing section disclosures

Exactly six direct accordion members use native `details` and the shared name
`event-horizon-sections`, in order: The protocol, 01 / Choose, 02 / Compose,
03 / Complete, Design Principles, Open deployment. `protocol-overview` contains the original
introduction and four ordinary semantic subsections: Open Source & Verifiable,
Engineered for Immutability, Perpetual Endowment Funding and Powered by Internet
Computer Protocol. Their original text, icons, links, fragment IDs and focus targets
remain. `protocol` and `protocol-title` still identify Design Principles and its four unchanged
principle articles. Hero and footer remain outside the group.

All start closed without a relevant fragment. Native grouping permits zero or one
open, including without JavaScript. Summaries retain pointer cursors, chevrons and
visible focus outlines, with no panel hover fill. CSS `::details-content` progressively
enhances native disclosure: 300ms ease-in-out intrinsic block-size interpolation
where supported. Other engines get a 280ms entry fade on the existing content
wrapper and clean native instantaneous collapse. WebKit cannot reliably finish a
descendant fade after native hiding; delaying its visibility can leave a blank
gap. This deliberate fallback avoids engine detection and custom animation
controllers. Engines without the pseudo-element keep native instantaneous behavior. Reduced motion disables
content and chevron transitions. No animation runs on initial page load.

`section-disclosures.js` reveals the owner of each section/control fragment.
Trusted pointer or Enter/Space summary activation pushes the selected section ID;
manual collapse clears only a fragment owned by that section, including nested
hero targets. Path/query and existing `history.state` are retained. Unchanged
fragments do not create duplicate entries. Native exclusivity remains authoritative;
unattributed toggles/data updates, programmatic reveals, resize/zoom and setup never
write history. A transient trusted-activation marker commits only its native toggle
result, independently of the cancellable scrolling frame.

`popstate` restores section selections or the zero-open state without rewriting
history; `hashchange` preserves ordinary native anchors. Hero fact links keep
`#open-source`, `#immutability`, `#endowment-funding` and `#internet-computer`.
Repeated links and Compose/Complete navigation continue working. Normal `#top`,
`#info` and skip links retain native meaning. Returning through history to a
no-section entry closes the group without changing its URL.

Initial URL and history restoration reveal/scroll without forced focus. Static
negative tabindex on hero explanation sections also caused native fragment focus;
these sections now receive temporary tabindex only for keyboard link activation,
removed on focusout. Keyboard summary/link outlines remain visible. Closing
focused hidden content still repairs focus to its summary. One cancellable,
bounded post-animation correction preserves deep-link geometry; later user focus
is respected. No general router, expansion-state store or scrolling loop is added.

All six accordion introductions use their available content width and wrap naturally, with unchanged typography. The four detailed Protocol subsections retain their 65ch editorial paragraph measure. Protocol heading icons alone use 32px boxes (ICP: 52px), centred against the entire multiline heading with flexbox. Scoped wrapping keeps long words inside the column with enlarged text; hero icons and artwork remain unchanged.

After esbuild, run `npm run test:frontend-unit`,
`node tools/scripts/validate-subnet-section.mjs --live --zoom`,
`node tools/scripts/validate-ledger-counter.mjs` and
`node tools/scripts/validate-subnet-example.mjs --live`. The subnet harness separates
deterministic cross-engine lifecycle/URL/focus/geometry/slow-load checks from genuine
Dashboard checks. `--zoom` adds actual Chromium browser zoom and genuine Dashboard
BFCache restoration. `--live-only`, `--output=directory`, `--no-screenshots`,
`SUBNET_BROWSERS` and `SUBNET_WEBKIT_EXECUTABLE` retain their existing meanings.

The live map check uses the existing local candidate-asset server; Dashboard and
ICP API traffic remain genuine. This fixture is not proof of a deployed revision.
URL/reload focus tests use deterministic API transport separately. WPE can report canceled `read_state` requests as access-control
errors during deliberate reloads; the same exact errors reproduce on the reviewed
production baseline. Fresh-session errors and all CSP errors remain strict checks.
PocketIC checks exact certified bytes/CSP and confirms example paths remain 404s.
See the [original performance report](../../docs/codex/navigation-widget-validation.md)
for measured results and limitations. Historical reports describe their original
revisions. WebKit automation does not establish physical Safari/device or manual
screen-reader testing. Wide proportional scaling can offset magnification until
the native narrow mode resumes; the external link supports independent zoom.

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
| Logical attempt deadline, including initialization | 30 seconds |
| Failure retry | 30, 60, then at most 120 seconds |
| Delayed indication | 3 minutes without an accepted sample |
| Inconsistent indication | Three lower responses since acceptance |

`ledger-counter.js` separates accepted total, displayed integer, target,
monotonic sample time, wall-clock receipt timestamp and status. BigInt arithmetic
preserves even generic totals above `u64::MAX`. The first sample is direct,
equal samples do not extend smoothing, and no value is extrapolated. Once the
latest target is reached there are no numeric animation frames. Digit grouping
preserves every integer; long values wrap rather than overflow or truncate.

The visible explanation describes smoothing lag and that **Last checked** means
accepted-response time, not proof of freshest replica state. Failures retain the
last known number and timestamp; stale status has its own timer. An established
count retries quietly through isolated failures; only a sustained three-minute
gap shows the delayed-data notice. Contradictory responses and configuration
mismatches retain their explicit indications. A permanent two-line notice slot
prevents status appearance/recovery from moving the bottom-aligned count and
label; unusually large text can still reflow normally. A lower sample
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
runtime ledger total. No proof-backed tip-certificate feature is claimed. The
original ledger-counter validation covered frontend assets and tests without
changing the backend. The current coordinated release follows the deployment
procedure linked below.

For deterministic tests, real-browser checks, opt-in read-only live smoke and
limitations, see [testing](../../docs/development/testing.md) and the
[ledger-counter validation report](../../docs/codex/ledger-counter-validation.md).

## Hero branding and sampled recent rate

The compact hero restores the art-first spacing used before the counter
(`41e088f`, following the layout in `63c4022`): the full heading, lead, CTA and
facts follow the cropped SVG's complete height. Only the ledger statistic stays
in its own lower-right artwork region, capped to the first viewport when the
art is taller. Its label, left-hand unbracketed rate and right-aligned count stay
unboxed. Narrow portrait layouts place the group close to the SVG's lower edge
while keeping its complete layout box inside the artwork; long/error content
expands the row rather than overlapping the body.
Wide desktop keeps its title/artwork composition and proximity to the fourth hero link.

The `EVH.` shortcut fits the dark corner in the compact art-first layout while the large
heading would obscure the artwork. Show it only if the full heading is actually
below the viewport or peeking into its bottom edge by at most 128 CSS pixels
(capped to three quarters of the heading height). Hide it beyond that tolerance or
when it leaves upward. Tall screens, scrolling, text zoom and reflow make the media
condition alone insufficient. The link uses the same compact-layout media rule (width at most 81.25rem OR
aspect ratio at most 1/1), not a separate landscape-only gate. It defaults hidden, uses viewport observation
and an initial geometry check, and fails closed without observation. Cleanup
removes observers/listeners/frames. Keep the comments and browser regression
checks when changing hero spacing; hide the compact mark once the full heading exceeds the documented allowance.
Resize/reflow events are coalesced before state changes, and the mark fades over
700ms instead of flashing between states. It becomes noninteractive and leaves
the accessibility tree as soon as hiding begins. Reduced motion disables the fade.
Its compact underline has short vertical shoulders before a full-width shallow
downward point. A paper-coloured core and separate warm halo mirror the lettering's
glow. The underline sits inside the anchor's click target and reinforces the title
link's direction without adding another accessible element. The clipped core is a
child of the element carrying the filter so the polygon cannot clip away its halo.
The resting lettering and arrow use a restrained glow; pointer hover and keyboard
focus enhance both together. Reduced motion applies the state change immediately.

The unboxed Ledger label, sampled rate and grouped total use layered dark text
shadows so they remain legible across the artwork without introducing a panel.
The small uppercase label has an extra dense shadow because it crosses the
brightest part of the artwork.

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

Permanent Historian identity aliases and explicit `subscriber@subject` declarations
are supported. Omitted subject means self. The canonical ICP payment prefix is
`E.`; retired `X` is not offered. Identity aliases are not general payout routes.
The builder shows resolved principals and validates alias declarations through
the selected backend's local `preview_subscription` query. See the
[subscriber guide](../../docs/subscriber-guide.md) for exact-route funding, unknown-cache admission, external-only callbacks,
local subject filtering, and authoritative reconciliation.

The current first-release sequence is owned by
[Deployment](../../docs/operations/deployment.md). After verifying the reviewed
backend, separately authorise this existing frontend's upgrade, compare its
installed module hash and verify certification and the backend pin. With no
subscriptions, these upgrades and observation are independent of Jupiter's
schedule. External alias, funding/admission and receiver compatibility are
required before funded subscription activation. Retain backend controller access
until the separate, stricter controller-removal requirements are satisfied.
