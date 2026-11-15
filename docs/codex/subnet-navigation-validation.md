# Subnet copy and embedded dashboard navigation

Baseline: clean mainline `700ba81431412ac4cd64a8095abab53afff159b8`, PR #17 merge,
confirmed against origin/main on 2026-11-14. This change is frontend-only and
performs no deployment, reinstall, migration, controller or governance action.

## Copy and placement

The semantic h2 “Fully on-chain”, aria-labelledby, two-column desktop layout,
stacked responsive layout and 520px map height are preserved. The description is
exactly:

> All Event Horizon canisters are hosted on one of the largest ICP subnets.

Fresh certificate-verified reads using the pinned SDK, its embedded mainnet root
key and normal freshness checks placed both deployed production canisters on
`pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae`, with 34 node
keys: backend `eo6ei-gaaaa-aaaar-qchra-cai` and frontend
`ej7c4-lyaaa-aaaar-qchrq-cai`. Their observed module hashes match the previous
reviewed production artifacts. The IO registry entry is planned with no deployed
canister ID and is excluded from this claim.

The dashboard's [current subnet table](https://dashboard.internetcomputer.org/network/subnets)
listed 42 mainnet subnets: one with 40 nodes, three with 34, 35 with 13, and three
with 7. The target is a Fiduciary subnet in the joint-second-largest listed group.
The static copy must be reviewed if another official instance deploys elsewhere
or subnet composition changes; no runtime discovery mechanism is introduced.

Removed the two standalone source links and their nav/div wrapper. The unchanged
attribution/fallback paragraph is now directly a p with class
subnet-section__sources. Removed the obsolete descendant nav and p CSS rules;
retained fallback-link styling. Historical validation reports describe their old
revisions and are intentionally retained. Loading JavaScript and its built bundle
are unchanged: the retained fallback precedes the slow status, so “link above”
remains correct. Native lazy loading, dark loading, 500ms settling, 12-second slow
explanation, late recovery and no-JavaScript fallback remain covered.

## Root cause and supported navigation

The deployed two-token sandbox reproduced a Chromium error that opening a new
window was blocked because allow-popups was not set. The real dashboard anchor
is a standard link to the exact subnet page followed by #node-placement, with
target="_blank" and rel="noopener noreferrer". This is new-tab navigation, not
an attempt to replace the Event Horizon top-level document.

The iframe's complete permissions are now:

```text
allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox
```

This follows the [dashboard embed guidance](https://dashboard.internetcomputer.org/methodology#embeddable-subnet-map)
and the [HTML sandbox model](https://html.spec.whatwg.org/multipage/iframe-embed-object.html#attr-iframe-sandbox).
The first added permission lets the anchor open a browsing context; the second
prevents sandbox inheritance in the new full dashboard page. The iframe retains
its sandbox, lacks top-navigation/form permissions and keeps its exact src,
loading="lazy" and referrerpolicy="no-referrer". Parent page and SVG CSP strings,
connection restrictions, production IDs and mappings are unchanged.

## Opener isolation and trust implications

The current anchor explicitly uses noopener and noreferrer. The real-browser
harness requires these attributes, an actual new tab at the expected destination,
a null JavaScript window.opener before and after destination-page interaction,
and an empty document.referrer. The null opener severs the handle used for
reverse-tabnabbing; the new tab cannot navigate or control the parent through
that relationship. See the [HTML noopener definition](https://html.spec.whatwg.org/multipage/links.html#link-type-noopener).

These permissions expand trust in the independently maintained dashboard's popup
behavior. Popups are normal external browsing contexts, outside Event Horizon's
CSP/certification. The permissions are neither a destination allowlist nor a
requirement that arbitrary future window.open calls use noopener. Browser popup
blocking still applies; the current user-initiated link is tested, not a guarantee
about every possible future dashboard script. Recheck the external integration
on releases. There is no parent click overlay, external DOM rewriting, message
protocol or duplicated dashboard navigation control.

## Validation and reproducible release evidence

The section harness retains all eight viewport checks, semantic heading/flow,
keyboard entry/exit, no overflow, enlarged text, reduced motion, forced colours,
controlled pending/slow/late/failed documents and no-JavaScript fallback. Real map
country selection and data-centre expansion remain covered on desktop, phone,
320px phone and landscape, including a dark-device desktop.

Its new real-dashboard tests activate the embedded anchor with desktop click,
keyboard Enter and emulated phone touch. The full destination page has its own
UI: a data-centre map and a paged node table. Tests verify the rendered subnet and
data-centre links, select ten rows and observe the updated live pagination. The
dashboard's own pagination updates its tab URL to ?nodes-s=10; the initial opened
URL is checked before that interaction. Event Horizon's URL, section markup and
iframe src stay unchanged, with no additional tab beyond the one activated.

Frontend unit assertions and the certified PocketIC HTML/asset test cover the new
copy, removed links, exact sandbox, retained fallback and absent top-navigation
permission. The applicable PocketIC scope is the two certified frontend serving
and cache/routing scenarios; backend source and interfaces do not change.

Run the established frontend build/unit and browser regressions, Rust workspace
unit/static matrix, formatting/Clippy, the full security scan and source manifest.
After committing, run the canonical two-clean-build workflow and retained artifact
build. The backend must match
`f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970`;
frontend hashes are recorded in the canonical receipts and artifact manifest.
No dependency lock or advisory exception changes are made.

Ignored receipts under observations/subnet-navigation retain research (blocked
navigation, certified placement and size comparison), browser evidence and popup
screenshots, frontend/unit/PocketIC/security logs, canonical build logs and final
GitHub CI results. Source documentation and MANIFEST.sha256 are committed; these
execution artifacts are kept outside the canonical git-archive build context.
