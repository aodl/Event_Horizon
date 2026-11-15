# Full-width subnet map experiment

## Starting point and scope

Started from clean `303ff4601fee3ed59e5a1a514633d86083a3016c` on
`codex/subnet-copy-navigation`, then created `codex/subnet-full-width-scaling`.
GitHub confirmed PR #18 was open/unmerged; origin/main remained
`700ba81431412ac4cd64a8095abab53afff159b8`. This experiment therefore includes
PR #18 and should be reviewed as a stacked change above it.

The HTML, wording, heading, attribution/fallback, exact subnet endpoint, lazy
loading, no-referrer policy and four sandbox tokens are unchanged. There is one
iframe, immediately before “The protocol”. No deployment, merge, backend,
interface, identity, controller, governance or deployment-mapping changes occur.

## Comparison and chosen prototype

The [current embed guidance](https://dashboard.internetcomputer.org/methodology#embeddable-subnet-map)
still provides no fluid-width option or dashboard-ready notification. Genuine
browser measurements confirm the internal card's 760px maximum. Increasing an
iframe from 640px to 1273px increases the card from 622px to only 760px. At 1697px
the natural card remains 760px. No undocumented parameters were added.

| Strategy | At 1440px viewport | At 1920px viewport | Assessment |
| --- | --- | --- | --- |
| PR #18, two columns | 640×520 iframe | 640×520 iframe | Compact, occupies a small part of the available content width |
| Natural full-width, vertical | 1273×520 | 1697×520 | Card remains 760px, large internal horizontal margins |
| Scale 640×520 baseline | 1273×1034 | 1697×1379 | Fills width but grows controls and vertical height more than necessary |
| Scale 760×520 baseline | 1273×871 | 1697×1161 | Fine-pointer prototype: about 97% plot occupancy; touch limit below |

### Touch-specific plot limit: a failing adoption criterion

The card and the geographical plot have separate constraints. In wide touch
emulation the outer card still grows to 742px inside the 758px viewport, but its
world SVG is only 584px wide. Thus the 760px prototype fills the iframe while the
actual plot uses about 77% of it. Short touch landscape uses about 79%. This was
caught by the added 90% actual-map occupancy assertion, which is deliberately
kept strict: the final live harness fails this adoption gate on scaled-touch.
The earlier functional run completed all eight contexts before this gate was
added; it does not override the negative occupancy result.

Test-only parent styling compared smaller baselines without changing the source,
sandbox or external DOM. Ratios below use the embedded layout viewport; the
iframe border accounts for a small difference from the outer visual ratio.

| Touch viewport | Internal baseline | Visual frame | Actual SVG / internal viewport |
| --- | --- | --- | --- |
| 1440×900 | 760×520 | 1273×871 | 77.0% |
| 1440×900 | 640×520 | 1273×1034 | 91.5% |
| 1440×900 | 600×520 | 1273×1103 | 97.0% |
| 844×390 | Native 746×520 | 746×520 | 78.5% |
| 844×390 | 640×520 | 746×606 | 91.5% |
| 844×390 | 600×520 | 746×647 | 97.0% |

Smaller baselines improve plot occupancy at the cost of still larger controls
and additional height. A single 760px baseline therefore does not satisfy the
width objective across the tested pointer modes. Neither the 640px nor 600px
touch alternative is silently substituted into the prototype. Adopting a
pointer-dependent baseline would need a separate design and accessibility review.

Touch comparison screenshots: [760px baseline](subnet-scaling-evidence/1440-touch-760.png)
and [600px baseline](subnet-scaling-evidence/1440-touch-600.png).

CSS zoom was also compared in Chromium. It affects layout geometry and produced
slightly different internal measurements (759px viewport instead of 758px at
the same nominal baseline). It is not selected; transform has explicit layout
and visual dimensions and does not rely on CSS zoom across engines.

The prototype uses a vertical composition within the existing gutters. The
unchanged attribution stays above the iframe, preserving keyboard order and the
slow message's “link above”. No changes to unrelated section spacing are made.

## Geometry and lifecycle

At available width W > 760px: fixed iframe layout size 760×520, visual scale
W/760, reserved wrapper height 520×W/760. Both axes use one transform with origin
top left. At W ≤ 760px: native responsive width W and 520px height, no transform.
The crossover is continuous; phone content is never shrunk from a desktop layout.

One ResizeObserver watches the parent width, not the transformed frame. It ignores
height-only notifications and subpixel noise, including an explicit check that
small changes across the crossover still switch modes. It applies geometry
before the opt-in data attribute, catches initialization/observer failures, and
disconnects and resets on disposal. There is no polling, continuous animation,
iframe recreation, source assignment, external DOM access or additional fetching.
Missing ResizeObserver, failed initialization and no-JavaScript cases retain a
normal untransformed full-width 520px iframe.

| Viewport | Visual iframe, CSS px | Mode | Whole section height, CSS px |
| --- | --- | --- | --- |
| 1920×1080 | 1697.27×1161.29 | Scaled | 1495.33 |
| 1440×900 | 1272.95×870.97 | Scaled | 1127.34 |
| 1280×800 | 1131.52×774.19 | Scaled | 1017.77 |
| 1024×768 | 905.20×619.35 | Scaled | 842.45 |
| 873×940 | 771.72×528.02 | Scaled | 749.22 |
| 768×1024 | 678.91×520 | Native | 741.20 |
| 390×844 | 350×520 | Native | 791.50 |
| 375×667 | 335×520 | Native | 791.50 |
| 320×568 | 280×520 | Native | 841.80 |
| 844×390 | 746.09×520 | Native | 741.20 |

The section becomes deliberately taller: the map fills the extra area, rather
than letterboxing. The 1440px section increases from about 636px to 1127px; at
1920px, about 675px to 1495px. The enlarged country labels are more prominent than
the parent copy. This is a design tradeoff, not a compact-layout improvement.

## Genuine browser zoom

An unpacked test-only Chrome extension calls chrome.tabs.setZoom/getZoom. This
is real desktop page zoom in full Chromium, not devicePixelRatio emulation, CSS
zoom or mobile pinch. The browser's effective CSS width and visualViewport.scale
(one) are recorded. Ordinary resizing is tested independently in both directions.

| Zoom at 1440×900 browser viewport | Effective CSS width | Visual frame, CSS px | Mode |
| --- | --- | --- | --- |
| 100% | 1440 | 1272.95×870.97 | Scaled |
| 125% | 1152 | 1018.36×696.77 | Scaled |
| 150% | 960 | 848.62×580.64 | Scaled |
| 175% | 823 | 727.40×520 | Native |
| 200% | 720 | 636.48×520 | Native |

Returning to 100% restores the original geometry with no iframe navigation. The
wide auto-fit offsets some browser magnification: map labels stay approximately
the same physical size while the wide scale decreases. Native mode then permits
normal magnification. Passing this reflow test is not a claim that map text grows
200% relative to its initially upscaled appearance. The permanent external tab
offers the dashboard's independent native layout and zoom.

## Interactions, accessibility and loading

The browser harness verifies visual getBoundingClientRect geometry separately
from the iframe's offsetWidth/offsetHeight. It checks reserved height, the next
section, horizontal overflow, stable repeated resizing and one document navigation.
It retains the heading/accessibility-tree, Tab/Shift+Tab, visible focus, enlarged
parent text, reduced-motion, forced-colour and no-JavaScript coverage.

Real dashboard tests use country selection, centre expansion, geographical SVG
markers, the information dialog, scrolling and user-initiated dashboard tabs.
Country panels and marker hover/pinned panels have different semantics and must
be dismissed before the next action. A one-centre country directly shows that
centre rather than the multi-centre list. Tests assert the appropriate genuine UI.

Playwright can report untransformed child boundingBox coordinates for an OOPIF.
Test-only instrumentation therefore uses standard child DOM rectangles and the
parent frame's measured transform to send trusted mouse/touch input. It verifies
the actual pointer event target and isTrusted. Coarse-pointer SVG hit circles can
be separate from the visible dot; the genuine data-centre dialog response is
checked. Touch uses the sheet’s own Close button. Keyboard activation uses native
focus and Enter/Space. Production code contains none of this instrumentation.

The loading surface is a sibling of the transformed iframe and covers its whole
reserved visual area at normal parent text size. Existing 500ms settling and
12-second near-frame slow recovery remain unchanged; resize never restarts them.
Neither iframe load nor a timer is treated as external data readiness.

## Browser compatibility limitation

Chromium 149 and Firefox 151 render the genuine map. Firefox passed the complete
resize matrix and a functional isolated external tab. Available WebKit 26.5/WPE
starts the genuine embed but its dashboard SvelteKit runtime rejects initialization:

```text
TypeError: undefined is not an object (evaluating 't.resetIndex')
MIYGXTpg.js:2:18592 / MIYGXTpg.js:2:20272
```

The same failure reproduces in an untouched PR #18 iframe, without scaling.
Public source inspection places it in history initialization immediately after
history.replaceState, before map controls render. Direct top-level navigation to
the same embed renders controls. This establishes an existing embedded-runtime
limitation in this test environment; it does not establish an actual Safari bug.
No third-party workaround, history patch, proxy or sandbox relaxation is applied.
Actual Safari and physical devices have not been tested.

## Evidence and release gates

The frontend setup/build, 147 frontend and 146 Rust unit tests (293 in the
established five-suite runner), formatting, Clippy, static/source checks and
security scan passed. The full managed PocketIC integration suite passed all 129
scenarios, including both certified frontend cases. The existing advisory
exceptions and locks are unchanged.
Hero/ledger regression passed. All ten viewport geometries and repeated resizing
passed; controlled cross-origin frames also verified geometry and trusted input
in Chromium, Firefox and WebKit at eleven sizes/transitions per engine. These
fixtures do not establish that the real WebKit dashboard works.

A genuine-dashboard performance probe recorded zero new parent or dashboard
requests through twelve resizes and no window/ResizeObserver errors. The normal
section harness independently checks one document navigation, no source changes,
no loading restart and no overlap. Canonical/hosted release results are recorded
in the post-commit receipt and PR rather than predicted here.

Ignored observations/subnet-scaling contains the strategy comparison, matched
baseline/natural/scaled screenshots, all responsive views, five genuine zoom
levels, pending/slow loading, expanded controls, browser receipts and full logs.
Representative comparison screenshots are retained beside this report for PR
review. Screenshots are inspected; dimension assertions alone are insufficient.
Fractional scaling can change glyph antialiasing; SVG geography remains sharp.

| View | PR #18 baseline | Natural full width | Scaled prototype |
| --- | --- | --- | --- |
| 1440×900 | [Baseline](subnet-scaling-evidence/1440-baseline.png) | [Natural](subnet-scaling-evidence/1440-natural.png) | [Scaled](subnet-scaling-evidence/1440-scaled.png) |
| 1920×1080 | [Baseline](subnet-scaling-evidence/1920-baseline.png) | [Natural](subnet-scaling-evidence/1920-natural.png) | [Scaled](subnet-scaling-evidence/1920-scaled.png) |

Native phone views: [390×844](subnet-scaling-evidence/390-native.png) and
[320×568](subnet-scaling-evidence/320-native.png). These are viewport screenshots,
so the tall map continues below the fold. Full-frame and expanded-control images
are in the ignored evidence folder. All three desktop strategies use the same
device dark preference; no theme parameter is added to the iframe URL.

Run the frontend build/unit, live section and zoom harness, hero/ledger browser
regression, Rust workspace tests, formatting/Clippy, static/security/source-manifest
checks and full managed PocketIC suite. After the final commit, run the canonical
two-clean-build comparison, retained artifact build and production export audit.
The final receipt and PR record source revision, test results, CI URLs and raw
Wasm hashes. The backend must remain
f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970.

## Recommendation

**Do not adopt this prototype.** The 760px baseline fills the fine-pointer plot
but fails the width goal in wide touch emulation. Smaller touch baselines improve
occupancy while increasing height and control size further. Wide auto-fit also
offsets browser text magnification. Genuine embedded WebKit validation remains blocked by the independently maintained
dashboard's history initialization failure, also present in the baseline. Obtain
an actual Safari integration result before treating this as merge-ready. Keep
the current deployed design; this branch is an isolated, reviewable experiment.
