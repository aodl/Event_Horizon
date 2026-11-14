# Subnet section layout and loading refinement

Starting revision: `d4f556f498e1be8a4b470ee3e24658b6e1e1672c`, confirmed
against `origin/main` on 2026-11-14 (PR #16 merge; clean checkout).
This is a frontend presentation change. No backend source, production configuration,
Candid, dependency lock, CSP, SVG policy, sandbox or deployment changes.

## Layout and live review

Reviewed the JavaScript-rendered [embed methodology](https://dashboard.internetcomputer.org/methodology#embeddable-subnet-map)
and [13 November changelog](https://dashboard.internetcomputer.org/changelog).
The documented parameters are subnet and theme; there is no fluid-width option or
supported readiness notification. The embed's own card is capped at 760px.
The documentation recommends 520px desktop and 560px phone height; 520px was
retained after testing this actual subnet's complete map, figures, country list,
data-centre disclosures and attribution. Longer lists and mobile detail sheets
use the dashboard's own scrolling. No cross-origin production DOM access,
postMessage protocol, invented parameters, cropping or scaling was added.
The precise reviewed endpoint remains unchanged.

“Fully on-chain” is now the small h2 and region name. The oversized heading is
removed. The section remains immediately before the unchanged manifesto.
At 1024px and wider, copy and attribution occupy the left column and the map the
right. Below that breakpoint, copy precedes the map. All frames are 520px high:

| Viewport | Frame width | Layout |
| --- | ---: | --- |
| 1440×900 | 640px | Two columns |
| 1024×768 | 640px | Two columns |
| 768×1024 | 640px | Stacked |
| 390×844 | 350px | Stacked |
| 375×667 | 335px | Stacked |
| 320×568 | 280px | Stacked |
| 844×390 | 640px | Stacked, ordinary page scrolling |

The 1440px section shrank from 1252px to 636px (49%); at 1024px it shrank from
1102px to 603px (45%). Padding is reduced only for this section and its boundary
with the manifesto. The thin frame uses existing colours, with no shadow or
outer promotional card. There are no floating, modal, minimise or close controls.

Screenshots were visually inspected, including loading, actual desktop/mobile map,
320px width and short landscape. The real dashboard has no horizontal document
overflow at these frame widths, including 278px of internal width on the narrowest
phone. Its mobile country detail sheet can cover the lower map until dismissed;
that is native dashboard behaviour, not parent cropping. Touch testing is emulated.
The unchanged endpoint follows device colour preference: light devices still get
the dashboard's light paper. Dark-device desktop is checked separately.

## Loading and limitations

The opaque `--space-soft` layer exists before application initialization and is
hidden by CSS when scripting is disabled. Its understated indicator and status
respect reduced motion. Document load starts a 500ms visual settling interval,
then `hidden` removes the layer from rendering and accessibility. No invisible
input-blocking layer remains. A visibility observer starts a 12-second slow-load
deadline near the lazy frame. The deadline removes the layer/indicator and shows
polite explanatory text beside the always available fallback links; late load
clears it. Disposing cancels timers, observer and event listener.

The deadline is not a failed-request diagnosis. Removing the layer also handles
a document load occurring before the bundle attaches its listener. The parent
cannot observe cross-origin application/data readiness. After settling or the
slow deadline the external page can still paint white, show its own error or
continue fetching registry/map data. No precise completion claim is made.
The original sandbox blocks embedded new-tab links; parent source links remain
usable without granting popups or weakening the reviewed boundary.

## Validation and retained evidence

- Frontend Node tests: 144 passing, including delayed loading, recovery and disposal.
- Controlled Chromium checks cover all seven required viewports plus 873×940,
  dark pending navigation, slow explanation, late recovery, actual fixture input,
  failed external request isolation, permanent fallback, scripting disabled,
  reduced motion, 200% text, forced colours and native keyboard entry/exit.
- Genuine dashboard checks exercise country selection and data-centre expansion
  on desktop, mobile, 320px phone and short landscape; dark-device desktop is also
  exercised. Successful load alone is never counted as map readiness.
- The hero/ledger regression harness retains its full checks. Its click-to-keyboard
  transition now waits for native smooth anchor navigation to finish before
  resetting scroll. The ledger unit test scopes its no-live-announcement assertion
  to the ledger section, allowing the independent loading status.
- `cargo fmt --all -- --check`, workspace Clippy with denied warnings and the
  established security scan pass. The scan retains only existing advisory policy;
  no exceptions were added. Locks preserve quinn-proto 0.11.19, quinn-udp 0.5.16,
  rustix 1.1.6 and Hyper 1.9.0; pin-utils remains absent.

Run receipts for the final committed revision are retained under ignored
`observations/subnet-section/`: `test-unit.log`, `pocketic.log`, `security.log`,
`reproducible.log` and `docker-build.log`. Browser screenshots, accessibility
snapshots, real controls, geometry and request records are in `browser/`;
`before/` retains the same baseline viewports and real integration screenshots.
`research/` retains the retrieved methodology/changelog and current DFINITY
static-site/security guidance consulted for the unchanged certification boundary.

Release gates run **after committing**, with a clean manifested source tree:
`cargo run -p xtask -- test_unit`, the serial PocketIC integration suite,
`tools/scripts/security-scan`, `tools/scripts/verify-reproducible-artifacts`
(two independent clean Docker builds), and `tools/scripts/docker-build`.
The canonical pipeline audits production exports and release artifact checksums.
The backend must remain byte-identical to
`f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`;
the changed frontend hash and exact source revision are printed in both canonical
receipts and retained with `release-artifacts/release-artifacts.sha256`.
The source manifest covers this report. No mainnet deployment, reinstall,
backend upgrade, controller action or migration is performed.
