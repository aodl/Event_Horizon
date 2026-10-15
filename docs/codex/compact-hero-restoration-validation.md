# Compact hero restoration validation — 2026-10-15

Validated working-tree changes on `main`, based on
`3e2493f526998daa2f8f7a2d4e5c08ff1e3120c5`. Earlier pending frontend edits were
preserved. This is local validation, not a deployed or canonical release revision.

## Historical intent and correction

`63c4022` placed compact text after the cropped SVG; `41e088f` still reserved
`80.398vw` for that artwork. `66cf0fa` added the smaller `EVH.` anchor because it
fits the dark corner without covering the ring. The counter's later shortened
spacing broke the separation between large text and artwork.

Compact layout now reserves the exact cropped ratio, `941 / (1672 * .7)` times
viewport width. The full heading, lead, CTA and facts follow that region. Only
the single ledger statistic occupies a separate lower-right artwork region,
capped to the initial viewport when the artwork is taller. Long content can grow
normal flow. Wide desktop keeps its existing composition.

`EVH.` defaults hidden and requires both compact-landscape eligibility and the
actual full heading being wholly below the viewport. Any partial heading
visibility, above-view heading, ineligible media or failed observation hides it.
Geometry is checked initially and through viewport/resize/reflow observation.
Scroll measurements are coalesced without toggling the focused link on every
scroll event, preserving native smooth anchor navigation. Teardown removes
observers, media/viewport listeners and pending frames.

The rationale is documented at the markup, CSS placement/show rules and JavaScript
controller, as well as in `SPEC.md` and the frontend README.

## Checks

- `npm run build:frontend`: passed; generated bundle rebuilt from source.
- `npm run test:frontend-unit`: 93 passed, zero failures. Four compact-heading
  tests cover partial visibility, reflow, viewport changes, observer failure,
  focus, cleanup and scroll/anchor stability.
- `python3 tools/static-check.py`: passed.
- `node tools/scripts/validate-ledger-counter.mjs`: passed using pinned Playwright
  Chromium 153, the actual built page and separate injected-query fixture.
- Source manifest regenerated and verified; staged whitespace check passed.

Browser evidence is ignored under `observations/ledger-counter-art-first/browser/`.
The matrix includes 1440×900, 1280×800, 1024×768, 1000×800, 844×390,
390×844, 360×640, 320×568, tall 1000×1000 and 200% text. Assertions check compact
body/facts below the SVG, count in the artwork region, no overflow or overlap,
no simultaneous visible titles, anchor/return-to-top behavior, and heading
reflow within the same eligible 1000×830 media condition. Screenshots include
`regression.png`, `landscape.png`, `phone.png`, `tall-title.png` and
`same-media-reflow.png`. Existing rate/error/selection/reduced-motion/SVG
suspension and cleanup browser checks also pass.

No mainnet actions occurred in this restoration pass. Backend, interfaces,
instance configuration and polling behavior were not changed. Other browser
engines and physical devices were not exercised. Full PocketIC, security and
canonical build gates were not rerun for this layout-only pass.
