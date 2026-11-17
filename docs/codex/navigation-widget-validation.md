# Navigation and on-demand subnet widget validation

Reviewed base: `003e785bd244ed02810039fa46db2d3ea573c9b4`. Exact candidate and merge
SHAs, CI URLs, canonical hashes, deployment exit status and installed-module checks
belong to `observations/navigation-widget/release-receipt.json` and the release PR.
This source report alone is not evidence of deployment.

## Interface and scope

Six native grouped details remain, in order: The protocol, 01 / Choose,
02 / Compose, 03 / Complete, Design, Open deployment. The four original hero
explanations remain ordinary Protocol subsections with their original unique
fragments, content, icons and links. Native details/name owns exclusivity and
permits zero open; no separate expansion store or router was introduced.

Trusted summary activation pushes its section fragment once. Manual collapse
clears only an owned fragment, including nested hero IDs. Path/query and existing
history.state survive. Popstate restores section or zero-open entries without
writing history. Native anchors, special page anchors and repeated nested links
retain their meaning. Programmatic reveals do not overwrite specific fragments.

Initial forced summary focus caused the warm focus-visible outline. Browser tests
also identified native focus from static negative tabindex on nested sections.
Restoration now reveals/scrolls without forcing focus. Keyboard links temporarily
make a subsection focusable, retain its outline and remove tabindex on focusout.
Native summary keyboard focus remains. Closing focused hidden content repairs
focus; the finite post-animation correction remains. No global outline removal.

Only Protocol's direct introduction removes its 65ch cap. Text, colour and font
are unchanged; the paragraph wraps naturally at narrow/enlarged widths. Existing
CSS disclosure animation remains: Chromium intrinsic-size motion with ease-in-out;
Firefox/WPE opening opacity with clean instantaneous collapse where height
interpolation is unsupported. Reduced motion remains instant.

## Widget and resource lifecycle

Fresh visits start Minimised: a 46px flush-left vertical tab with Restore/Maximise.
Thumbnail has Minimise/Maximise, a 600×520 iframe at .25 (150×130 visible pixels),
and a 204px toolbar/frame. Expanded has Minimise/Restore and uses
min(66.667vw,1200px), switching to safe-area available width below 900px.
Exactly two state controls are visible. Short/zoomed panels scroll without reducing
44px targets; state changes return the panel's toolbar to view. Thumbnail overflow
contains the untransformed iframe layout, preserving its complete scaled image.
The Minimise target stays above the preview hit surface in constrained views;
browser tests sample its upper, centre and lower edges at short/zoomed sizes.
Short landscape layouts reserve space for the existing compact EVH shortcut.

Non-modal transitions keep the dialog open. Modal transitions use native
close/showModal/show with the same mounted iframe. Escape restores the previous
non-modal mode; direct #subnet-map defaults to Thumbnail on Escape. Widget controls
leave URLs unchanged; fresh navigation to #subnet-map opens Expanded.

No Dashboard src, loading timer, script or request starts before first
Restore/Maximise. That action starts one external document navigation; subsequent
state changes retain its DOM node, document time origin and country selection.
A full reload restores the unloaded Minimised default; BFCache keeps existing
valid presentation and the loaded document instead of initializing again.

The loader ignores empty-src events without cross-origin Document access. The
500ms document settling and 12s slow-load/late-recovery behavior remain. Document
load is not data readiness: browser checks wait for the milestone, then verify
real country/data-centre controls. Minimise hides rendering but does not guarantee
suspension of external JavaScript or network work.

## Performance

Paired cold-context traces and Performance API observations compare the reviewed
automatically loaded thumbnail with the on-demand candidate at 1440×900 and
390×844, three runs each. Instrumentation is development-only. No hero asset,
ledger polling/rate, operating refresh, pricing or preview logic changed.

Chromium 153, sequential cold contexts on the same shared host, without CPU or
network throttling. Medians below use three runs per viewport. Body-byte counts
are completed responses in the first five seconds, not a production compression
estimate. Hero readiness means the existing loaded/opacity milestone.

| Metric | 1440×900 baseline → candidate | 390×844 baseline → candidate |
| --- | --- | --- |
| Dashboard requests, first 5s | 148 → 0 | 175 → 0 |
| Dashboard body bytes, first 5s | 1,409,755 → 0 | 1,604,783 → 0 |
| DOMContentLoaded, ms | 731.5 → 718.1 | 359.6 → 354.4 |
| First contentful paint, ms | 660 → 648 | 280 → 244 |
| Hero ready, ms | 4160.4 → 3028.6 | 2046.9 → 1884.2 |
| Long-task excess over 50ms in first 2s, ms | 837 → 422 | 629 → 289 |
| First trusted click input queue delay, ms | 9.0 → 9.5 | 3.9 → 3.7 |
| First trusted click to next RAF, ms | 403.0 → 613.9 | 31.7 → 108.6 |
| Scroll-phase RAF interval p95, ms | 583.2 → 500.0 | 100.0 → 50.0 |

All six candidate runs retained an unset src and zero Dashboard requests before
activation (observed at roughly 9–15s), then made exactly one embed document
request. Minimized five-second samples produced zero new requests/long tasks and
0–1 additional iframe RAF callbacks; that does not prove suspension of scripts.

Startup/network and early long-task results support demand loading. Presentation
metrics are mixed: click-to-RAF did not improve. These headless rendering samples
are neither INP nor physical-device FPS. In a representative 1.5s click window,
main-thread FunctionCall time was 12.9ms baseline versus 4.3ms candidate; long
presentation delays therefore do not establish a new JavaScript handler bottleneck.
No claim of a general input/paint speedup is made, and no hero-quality or live-data
trade-off was retained. Synchronous link reveal preserves native fragment order
and prevents a queued earlier summary from overwriting a newer anchor; it is a
navigation correctness change, not a claimed performance optimization.

## Independent example

`examples/subnet-map-widget/{index.html,subnet-map-widget.js,subnet-map-widget.css,README.md}`
is an independent adaptation with one initializer/disposer, no application imports,
backend configuration, dependency or build system. It validates principal base32
encoding and CRC32, then safely encodes it under the fixed Dashboard origin.
One widget per page is intentional. It is neither imported into the production
bundle nor served/certified by the frontend; certified-response tests require 404
for example paths.

No licence was added. Public visibility grants no unrestricted reuse rights;
MIT/0BSD is proposed only subject to explicit owner approval. This outstanding
distribution decision does not block production functionality.

## Validation and limitations

Frontend setup, 164 Node unit tests, 146 Rust unit tests and all 129 managed
PocketIC scenarios passed with zero failures. Formatting, strict Clippy, static
checks and the security scan passed; the four existing documented Rust advisory
waivers remain unchanged. Certified frontend scenarios are repeated after the
final bundle. Final exact-SHA CI, reproducibility and release outcomes are recorded
in the release receipt rather than inferred from source checks.

Chromium 153, Firefox 155 and WPE/WebKit 26.6 exercised the eight requested viewport
sizes, URL/history, initial/reload focus, keyboard/pointer interaction, intro
width, reduced motion, forced colours, enlarged text, slow loading and widget
lifecycle. Actual Chromium browser zoom at 100/150/200% includes short landscape;
keyboard activation avoids automation pointer-coordinate ambiguity at native
zoom. All three widget modes retain the iframe through actual BFCache restores.
Normal pointer/emulated-touch checks are separate from the zoom checks.

Genuine Dashboard checks exercise country selection, data-centre/geographic
controls and isolated popup navigation. Candidate assets use the local server; API and Dashboard traffic are unmodified. This is not a
production-certification smoke test. Deterministic URL/reload tests remain separate;
the intercepted widget fixture starts in a fresh browser context after those cases
to isolate its frame association from earlier SVG-object reload/history state.
WPE access-control errors from canceled read_state requests during deliberate rapid
reloads also reproduce against the reviewed public baseline; normal fresh-session
checks and all CSP errors remain strict. No workaround for third-party code was
introduced.

The independent empty-page example passes Chromium/Firefox/WebKit fixtures and a
genuine Dashboard smoke check, with zero initial requests, one first navigation,
retained state and a null popup opener. Physical Safari/devices and manual
screen-reader testing are unavailable; WebKit automation is not physical Safari.

Backend source, Candid, page/SVG CSP, dependencies, deployment mappings and
controllers are unchanged. The required backend artifact remains
`f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`.
There is no new runtime telemetry, dashboard DOM manipulation, custom focus trap,
polling loop, UI framework or responsive engine. Presentation is one data-mode;
previousMode exists only for Escape. One layout observer and one iframe suffice.

## Changed source files

- `MANIFEST.sha256`
- `README.md`
- `canisters/frontend/README.md`
- `canisters/frontend/public/app.bundle.js`
- `canisters/frontend/public/index.html`
- `canisters/frontend/public/section-disclosures.js`
- `canisters/frontend/public/styles.css`
- `canisters/frontend/public/subnet-layout.js`
- `canisters/frontend/public/subnet-loading.js`
- `canisters/frontend/public/subnet-widget.js`
- `canisters/frontend/test/generic.test.mjs`
- `canisters/frontend/test/hero-facts.test.mjs`
- `canisters/frontend/test/section-disclosures.test.mjs`
- `canisters/frontend/test/subnet-section.test.mjs`
- `docs/architecture/trust-model.md`
- `docs/codex/navigation-widget-validation.md`
- `docs/development/testing.md`
- `examples/subnet-map-widget/README.md`
- `examples/subnet-map-widget/index.html`
- `examples/subnet-map-widget/subnet-map-widget.css`
- `examples/subnet-map-widget/subnet-map-widget.js`
- `tests/pocketic/src/lib.rs`
- `tools/scripts/validate-ledger-counter.mjs`
- `tools/scripts/validate-subnet-example.mjs`
- `tools/scripts/validate-subnet-section.mjs`
- `tools/static-check.py`
