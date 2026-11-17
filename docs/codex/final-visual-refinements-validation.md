# Final frontend visual refinements validation

Reviewed baseline: `8330308c3599610e7c80682a7dc6675ff43d69af`.
Exact source/merge SHAs, canonical hashes, CI/deployment results and public smoke evidence are recorded in the PR and `observations/final-visual/release-receipt.json`; this report alone does not establish deployment.

The non-modal widget uses a 50px bottom offset, 38px above its previous placement, respecting safe areas and falling back to 12px below 321px viewport height. The expanded modal geometry is unchanged. Minimised exposes one full-surface native button that restores Thumbnail. Thumbnail/Expanded retain their two destination controls; returning to Minimised focuses the tab. The controller's previous-mode Escape logic is retained, although the visible UI now enters Expanded through Thumbnail (direct #subnet-map still defaults Escape to Thumbnail).

All headings are visually uppercase with consistent weight/tracking; normal toolbars have 14px left padding and a 224px thumbnail frame accommodates the title and full control targets. Normal loading displays one centred CSS spinner, with visually hidden status text, a static reduced-motion ring, and the unchanged 500ms settle / 12s explanation / late recovery. The iframe URL is still assigned only on first use, and one mounted frame/observer is retained.

All six introduction paragraphs use natural block width; detailed Protocol paragraphs retain 65ch. Design Principles preserves protocol/protocol-title and all four principle articles. Protocol-only icons use 32px boxes (ICP: 52px), centred against the full heading with flexbox, with scoped overflow wrapping for enlarged text; hero icon dimensions and artwork bytes are unchanged. ICO inspection confirms favicon entries through 64px and Jupiter's native 32px entry.

The independent standalone example mirrors the presentation and controls without production imports. Its principal validation, origin, sandbox and CSP guidance are unchanged. Licensing remains pending owner approval; no licence grant was added.

Validation covers the maintained unit, Rust, managed PocketIC, static/security, browser and standalone suites; eight viewport sizes; actual Chromium zoom, keyboard/pointer/touch, reduced motion/forced colours, fragment/history and retained iframe lifecycle. Browser checks measure tab hit targets, offsets, spinner centring, all six intro widths and icon/text bounding-box centres. Held-request fixtures use independent browser contexts to avoid inheriting prior navigation/frame-driver state; genuine Dashboard traffic is unmodified. Hit-target checks wait for the initial fixture document; separate held-request cases exercise mode changes while loading. Results, counts, screenshots and any retries are retained in the release receipt. Physical Safari/devices and manual screen-reader testing remain unavailable.

No backend, Candid, dependency, mapping, controller, ledger/subscription, hero-art or security-policy change is intended. Only the existing tab destination selector changes in production JavaScript; presentation remains CSS/native HTML. No new observer, state manager, router, scroll tracking or runtime dependency is introduced.

The loading overlay is a sibling of the inert frame wrapper, so its visually hidden status remains accessible in Thumbnail as well as Expanded. It is positioned over the same map region without changing the iframe or loading controller.

At effective viewport widths of 200px or less, the expanded toolbar gives its title a separate row, preserving both 44px controls without overlap. Actual Chromium zoom validation covers all eight requested window sizes at 100%, 150% and 200%.

Changed source files (generated bundle and manifest included):

- `MANIFEST.sha256`
- `README.md`
- `canisters/frontend/README.md`
- `canisters/frontend/public/app.bundle.js`
- `canisters/frontend/public/index.html`
- `canisters/frontend/public/styles.css`
- `canisters/frontend/public/subnet-widget.js`
- `canisters/frontend/test/hero-facts.test.mjs`
- `canisters/frontend/test/section-disclosures.test.mjs`
- `canisters/frontend/test/subnet-section.test.mjs`
- `docs/architecture/trust-model.md`
- `docs/codex/final-visual-refinements-validation.md`
- `examples/subnet-map-widget/README.md`
- `examples/subnet-map-widget/subnet-map-widget.css`
- `examples/subnet-map-widget/subnet-map-widget.js`
- `tests/pocketic/src/lib.rs`
- `tools/scripts/validate-subnet-example.mjs`
- `tools/scripts/validate-subnet-section.mjs`
