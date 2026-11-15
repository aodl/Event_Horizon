# Exclusive landing sections — validation

Starting point: `main` / `99f67a1f06b3816224950acc8b9a1e8495892765`.
This candidate is a frontend interaction change; nothing is merged or deployed.

## Implementation

The 11 direct content sections use native `details` with one shared name and no
initial `open` attributes. Each first-child summary contains a single h2, existing
label/icon and a decorative chevron. Paragraph links remain outside summaries.
The native `open` property is the only expansion state; zero open is permitted.
Order: Protocol overview, Fully on-chain, Choose, Compose, Complete, Principles,
Open Source, Immutability, Endowment Funding, Internet Computer, Open deployment.
The former hosting sentence is absent. Existing IDs remain; `protocol-overview`
is new and does not replace `protocol`.

The 57-line fragment helper reveals the owning disclosure before normal anchor
navigation, including initial links, repeated fragments, nested controls and
Back/Forward. Section targets focus the summary without additional focus scrolling.
Unknown/malformed fragments are harmless. Modified, external, download and
new-tab links retain normal behavior. Ordinary toggles never change the URL.
A demonstrated Chromium defect moved Choose from y≈387 to y≈−589 when the tall
map closed above it. One queued check now scrolls that activated summary into view
only if it has left the viewport (observed y≈0 afterward). No scroll manager,
parallel expansion state, storage or compatibility accordion is introduced.

Forms and iframe remain mounted. The existing 45-line map layout controller and
loading implementation are unchanged: one ResizeObserver, 760×520 baseline,
proportional upscaling above 760px, native narrow layout, 500ms settling and a
12-second visibility-triggered slow explanation. Existing CSP, approved iframe
URL, sandbox/popup permissions, no-referrer and fallback link are retained.

## Results

- 152 frontend unit tests; 146 Rust unit tests; both certified frontend PocketIC
  scenarios pass. Certified HTML assertions cover the new order, disclosure
  membership, removed sentence, approved URL and retained security attributes.
- Frontend setup/build, strict formatting and Clippy, static checks and dependency
  security scan pass. The generated bundle comes from the established build.
- Both maintained browser harnesses pass. Native Enter/Space, zero/one open,
  visible focus, hidden-body Tab exclusion, initial/nested/repeated fragments,
  all hero and workflow links, history and skip links are covered.
- The injected read-only workflow fixture retains principal, amount, account and
  range/mode, selected instance and generated memo across Compose/Complete. Delayed identity preview completes
  while Principles stays open, then appears in Complete without resetting inputs.
- First map opening, hidden resize, reopening and rapid toggles preserve correct
  geometry and the same iframe document. The initial collapsed interval exceeds
  12 seconds without consuming the slow deadline. Pending dark cover, slow
  explanation, recovery while another panel remains open and fallback pass.
- No-JavaScript native operation, missing/failing ResizeObserver and failed
  fragment-initializer isolation pass. Browser-cache restoration was exercised
  with Chromium's cache-disable automation flag removed: `pageshow.persisted`
  was true and Compose remained open. No state-reset handler runs on restoration.
- Chromium 149.0.7827.55, Firefox 151.0 and WebKit/WPE 26.5 independently pass
  native/no-JavaScript grouping and keyboard operation; all 11 level-two headings
  are discoverable in their accessibility snapshots. Chromium's platform tree
  also exposes the native disclosure name and expanded state.
- Genuine dashboard mouse/keyboard navigation, country/data-centre controls,
  geographic markers, parent scrolling and mobile/wide-touch interaction pass.
  Its full-dashboard link opens the correct subnet in an independent functional
  tab with null opener; Event Horizon and the iframe remain intact.
- Actual headed Chromium page zoom (virtual display, `chrome.tabs.setZoom`, not
  DPR or CSS zoom) passes 100%, 125%, 150%, 175%, 200% and return to 100% after
  observer delivery, with no iframe reloads. Enlarged text, reduced motion,
  forced colours and short landscape geometry pass.

| Viewport | Visible iframe (CSS px) |
|---|---:|
| 1920×1080 | 1697.3×1161.3 |
| 1440×900 | 1273.0×871.0 |
| 1280×800 | 1131.5×774.2 |
| 1024×768 | 905.2×619.3 |
| 768×1024 | 678.9×520 |
| 390×844 | 350×520 |
| 320×568 | 280×520 |
| 844×390 | 746.1×520 |

There is no horizontal overflow or overlap with following sections. Browser zoom
uses the actual available width; headed scrollbars can reduce it slightly.
The intentional taller map and accepted 77–79% wide-touch geographic occupancy
remain unchanged; neither a maximum height nor a new occupancy threshold is added.

## Review evidence and release receipts

Five representative captures were visually inspected:
[collapsed](section-disclosures-evidence/collapsed.png),
[Protocol open](section-disclosures-evidence/protocol.png),
[real map open](section-disclosures-evidence/map.png),
[Compose open](section-disclosures-evidence/compose.png), and
[390px mobile](section-disclosures-evidence/mobile.png).
The form captures use offline configuration failure; injected valid workflow
results are tested separately. Additional execution evidence stays ignored under
`observations/section-disclosures/` and `observations/hero-facts/browser/`.

Canonical reproducibility, retained artifacts, export audits, the full managed
PocketIC suite and exact-head CI run after this source is committed. Their final
receipts and Wasm hashes belong to the PR release evidence, not an additional
application revision. The backend must remain
`f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`.

The earlier genuine-dashboard WebKit/WPE history-initialization issue remains a
separate external integration limitation; this run's WPE disclosure checks use a
stubbed frame. Actual Safari, physical devices and manual screen-reader testing
are not claimed. Actual Chromium Find (Ctrl+F with OS keyboard input on a virtual display)
opened the initially closed Endowment Funding section for the unique phrase
“Patrons can fund subscription declarations”. `window.find` is not used as a
substitute for the browser Find UI. No custom search is added.

Complexity: native markup replaces large section padding/direct-child layout
assumptions; obsolete subnet description and map-before-Protocol spacing are
removed. One small disposable helper owns fragment listeners and one queued
position check. No dependency, observer, backend request, iframe lifecycle or
application state store is added. Historical reports are retained.
