# Full-width subnet scaling validation

## Selected design and revised acceptance

PR #19 starts above PR #18 (`303ff4601fee3ed59e5a1a514633d86083a3016c`),
which was open/unmerged when this cleanup began. The experiment was
`18ac017c90de45b6ddcbed77a238b57456ff81c2`. Its HTML, exact copy, attribution,
subnet endpoint, lazy loading, no-referrer policy, popup permissions and CSP are
preserved. There is one genuine iframe immediately before “The protocol”.

The operator now accepts proportional height and the dashboard's wide-touch
layout. The former compact-height and 90% SVG-occupancy criteria are superseded.
The experiment really did fail its strict occupancy assertion; that result is
not reclassified as a pass. Its original measurements remain useful:

- Fine-pointer plot occupancy: approximately 97%.
- Wide touch plot occupancy: approximately 77–79%; a 584px world SVG can sit in a
  758px internal viewport even though the surrounding dashboard card is wider.
- A naturally wider unscaled iframe still leaves the dashboard card capped at
  approximately 760px, producing internal side margins.

The accepted implementation uniformly scales a 760×520px iframe when available
container width W exceeds 760px. Scale is W/760; the wrapper reserves
520×W/760 pixels of height. At W ≤ 760px the iframe is native W×520px, without
shrinking a desktop layout to phone size. Existing gutters and normal flow remain.
The added desktop height is intentional. No maximum-height cap, cropping,
pointer-specific baseline, external DOM modification or new dependency is used.

| Viewport | Visual iframe, CSS px | Total section height, CSS px |
| --- | --- | --- |
| 1920×1080 | 1697×1161 | ~1495 |
| 1440×900 | 1273×871 | ~1127 |
| 1280×800 | 1132×774 | ~1018 |
| 1024×768 | 905×619 | ~842 |
| 768×1024 | 679×520 | ~741 |
| 390×844 | 350×520 | ~792 |
| 320×568 | 280×520 | ~842 |
| 844×390 | 746×520 | ~741 |

These rounded measurements originated in the experiment; the final harness
checks current visual geometry with getBoundingClientRect, independently of
untransformed offset dimensions. The section was previously about 636px tall at
1440px. Representative inspected images: [previous layout](subnet-scaling-evidence/1440-baseline.png)
and [selected full-width layout](subnet-scaling-evidence/1440-scaled.png).

## Cleanup inventory and complexity assessment

| PR #19 change | Disposition |
| --- | --- |
| 45-line subnet-layout.js, app integration and generated bundle | Retained unchanged: necessary production functionality |
| Proportional CSS | Retained; one redundant scaled-height declaration removed because the shared iframe rule already sets 520px |
| Source manifest | Regenerated for documentation, CSS and removed evidence assets |
| Geometry, observer/disposal, loading and security unit tests | Retained; invalid observer measurements also checked |
| Browser harness | Kept focused geometry and live smoke paths; removed zoom/engine research, obsolete flags and repeated screenshots/sequences |
| README, trust-model paragraph and this report | Updated to describe accepted behavior and remaining limits |
| Ten experiment PNGs | Retained one before/after pair; removed eight alternatives/duplicates |
| One-off browser profiles, extension generation, source copies and comparison utilities | Not runtime dependencies; removed from the maintained change and local temporary evidence workspace |

The permanent harness no longer generates a Chrome extension, runs engine
research matrices or measures dashboard card ancestry. It checks actual iframe
width, full reserved height, map containment and usable controls. It deliberately
does not require wide-touch geographic occupancy to exceed 90%.

Production remains two functions: a pure geometry calculation and its disposable
initializer. One width observer is necessary; previous width prevents redundant
writes, and a disposed guard rejects late callbacks. Invalid dimensions are
ignored. Missing observer and partial initialization failure retain/reset native
layout. Existing loading observation is separate and unchanged. There is no
polling, animation loop, new listener, iframe recreation/source assignment or
runtime cross-origin access. No further production abstraction or refactor was
needed. The generated bundle is unchanged. Removing the duplicate CSS declaration
changes a certified asset, so the frontend Wasm may differ from the experiment
even though geometry and runtime JavaScript are unchanged.

## Maintained validation

Run frontend setup/build/unit tests, then:

```sh
node tools/scripts/validate-subnet-section.mjs --live
node tools/scripts/validate-ledger-counter.mjs
```

Optional harness flags are only `--output=directory` and `--no-screenshots`.
Without `--live`, the mocked path checks eight viewports, crossover/resizing,
no reloads/overflow/overlap, native Tab/Shift+Tab and visible focus, enlarged
parent text, forced colours, loader coverage/reduced motion, slow and late
recovery, failed requests, no JavaScript, and missing/failing ResizeObserver.
The shared live path covers fine-pointer desktop, scaled touch and native mobile.
It checks country/data-centre controls, genuine geographic markers and mobile
sheets, scrolling, Enter/Space, and the built-in full-dashboard link through
mouse, keyboard and touch. New tabs must have the correct destination, explicit
noopener/noreferrer, null window.opener, functional pagination and unchanged parent.

A small trusted-input helper accounts for transformed cross-origin iframe
coordinates in browser automation. Marker selection uses standard SVG geometry
because the external marker has no semantic button locator; response assertions
use the dashboard's visible data-centre UI. Production contains no such inspection.
No generated dashboard CSS classes or undocumented message APIs are dependencies.

Existing loading remains document load plus 500ms settling, with a 12-second
slow explanation and permanent fallback. Resize never restarts it. The overlay
covers the full visual frame at ordinary parent text size. Neither load nor a
timer proves the external application's data readiness; later white paints or
external failures cannot be detected precisely by the parent.

## Browser limits and release receipts

Actual Chromium desktop page zoom was exercised at 100/125/150/175/200% and back
to 100%, independently of ordinary resizing. It is not DPR, CSS zoom or pinch
emulation. Geometry remains stable without iframe navigation. Width-fitting
partly offsets magnification while scaled; native mode permits normal browser
magnification. The permanent external dashboard offers independent native zoom.
Parent text enlargement and automated accessibility checks do not establish
manual screen-reader or physical-device compatibility.

The experiment's genuine WebKit 26.5/WPE embed failed SvelteKit history
initialization (`t.resetIndex`), also reproduced against untouched PR #18. This
is an existing external embedded-runtime finding in that environment, not an
established scaling regression or actual Safari defect. No history workaround,
proxy or sandbox relaxation is introduced. Actual Safari is unavailable here;
physical Safari/device verification remains an operator acceptance consideration.

Cleanup validation passed frontend setup/build and 147 frontend tests; 146 Rust
workspace tests, formatting, Clippy and static checks; the unchanged dependency
security gate; both certified frontend PocketIC scenarios; the eight-view geometry
matrix and all three live smoke contexts. Hero/ledger regression passed on repeat
after an initial compact-heading scroll-reset timeout, without changing assertions.
The final CSS was also checked at genuine 100/125/150/175/200% browser zoom and
back. A short compositor-settling interval was needed in the disposable zoom
verification utility; it is not runtime logic or a permanent harness feature.
Final screenshots were visually inspected. The maintained harness is 417 lines,
down from 578; only two representative PNGs remain committed.

Final source-specific test, security, CI and canonical build receipts are recorded
in the PR and ignored observations/subnet-production, so post-commit hashes do
not require changing the revision being verified. Require explicit exit statuses
for the canonical comparison and retained build; the experiment's enclosing
143 exit is not a clean end-to-end success. Verify the artifact manifest and
production exports. The backend must remain
f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970.

**Recommendation: ready for operator review under the revised design decisions**,
with Safari/device verification clearly outstanding. PR #18 must land first;
then PR #19 can be retargeted to main so its eventual diff contains only scaling.
No automatic merge, deployment, backend/configuration change or migration is authorised.
