# Integrated subnet section and dependency security validation

This replaces the floating-widget design validated in
[subnet-widget-validation.md](subnet-widget-validation.md), preserving that report
as historical evidence. No deployment is authorised or performed by this change.

## Design and cleanup

The first section inside `main`, immediately before the unchanged “The protocol”
manifesto, presents “Fully on-chain”, a heading, one explanatory paragraph, the
real map and permanent attribution/source/fallback links. It shares `--content`
and `--gutter` with the page. A thin frame uses the existing palette and typography;
there are no fake locations, decorative network assets, cards or animations.
Desktop frame height is `clamp(600px, 72vh, 750px)`; widths at most 44rem use 520px.
Short landscape screens scroll normally. The following manifesto has a restrained
4–7rem top spacing rather than an additional large empty section gap.

Deleted `public/subnet-widget.js`, `test/subnet-widget.test.mjs` and
`tools/scripts/validate-subnet-widget.mjs`. Removed the app import, initializer,
exception handler and disposal hook; removed the floating aside, SVG, dialog,
controls, focus/scroll locking and every legacy CSS selector. Rebuilt the production
bundle with esbuild. The remaining app lifecycle and core queries are unchanged.

One static iframe has its exact reviewed HTTPS URL directly in HTML and native
`loading="lazy"`. Browser loading thresholds vary: zero initial dashboard requests
is not guaranteed. There is no custom observer, iframe lifecycle, polling or
continuous map thumbnail. The loaded dashboard stays in the document until
navigation. Simplicity and a dominant integrated map supersede the former thumbnail.

## Network placement and trust

Certificate-checked SDK mainnet reads at 2026-11-13 16:51 UTC placed both
`eo6ei-gaaaa-aaaar-qchra-cai` (backend) and `ej7c4-lyaaa-aaaar-qchrq-cai`
(frontend) on:

```text
pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae
```

The exact embed is
`https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae`.
Source links use `/network/subnet/<id>` and
`/methodology#embeddable-subnet-map`. Review the static relationship after a subnet
move. There is no runtime discovery.

Event Horizon certifies its HTML, CSS, JavaScript and response headers. The external
dashboard's independently maintained JavaScript and registry-derived data are
trusted for display and outside that certification tree. A placement map proves
neither source/Wasm correspondence nor controller removal. The frontend remains
independently upgradeable. No backend interfaces or protocol behavior change.

The sandbox remains `allow-scripts allow-same-origin`, with no-referrer and no
forms, popups or top-navigation grants. There is no iframe DOM access in production,
message protocol or proxy. Permanent fallback links cover failures the parent
cannot detect reliably. No availability monitor is introduced.

## CSP and browser evidence

The page and SVG CSP strings are unchanged. In particular:

```text
frame-src 'self' https://dashboard.internetcomputer.org
frame-ancestors 'none'
connect-src 'self' https://icp-api.io
```

All prior default, script, style, image, object and base restrictions remain.
Same-origin frames preserve the existing SVG object integration. The dashboard's
own subrequests do not require broadening Event Horizon's connection allowlist.

The live embed returned HTTP 200, no X-Frame-Options, and a frame-ancestors policy
permitting HTTPS and local browser testing. Browser validation serves the exact
Rust policies as response headers; PocketIC tests check actual certified responses.
HTTP success alone is not counted as successful rendering.

Browser evidence is written to ignored `observations/subnet-section/browser/`:
viewport screenshots, live desktop/mobile/narrow/landscape screenshots,
accessibility snapshots, response headers, requests, errors and geometry. The
harness uses the production bundle and independently mocked dashboard documents
for deterministic layout/keyboard checks; `--live` separately exercises the real
map's heading, country controls, data-centre details, touch and parent scrolling.
No assertions impose an exact native-lazy request boundary.

## Dependency remediation

Investigated the failed [Dependency Security run](https://github.com/aodl/Event_Horizon/actions/runs/43459724842)
and current RustSec database `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` and OSV.
The narrow lockfile updates are:

| Package | Before | After | Advisory |
| --- | --- | --- | --- |
| quinn-proto | 0.11.18 | 0.11.19 | RUSTSEC-2026-0523, RUSTSEC-2026-0524 |
| quinn-udp | 0.5.14 | 0.5.16 | RUSTSEC-2026-0538 |
| rustix | 1.1.4 | 1.1.6 | RUSTSEC-2026-0486 |
| hyper | 1.8.1 | 1.9.0 | Removes pin-utils 0.1.0 / RUSTSEC-2026-0441 |

The locked paths are PocketIC 15.0.0 → reqwest → quinn → quinn-proto/quinn-udp;
PocketIC → tempfile → rustix; PocketIC → reqwest → hyper → pin-utils (before).
Both integration and cadence-calibration packages use PocketIC. QUIC is an optional
locked transport rather than an enabled dependency in the default test graph.
The advisory scanner checks the entire lockfile, including that transport.
Windows dependency consolidation is a consequence of these patched crates.

Production backend and frontend normal/build dependency trees for
`wasm32-unknown-unknown` compare identically with reviewed mainline. No production
manifest or toolchain changes. PocketIC stays at 15.0.0. The pin-utils finding is
maintenance status, not a demonstrated exploit; removing it requires no exception.
No security policy was weakened. Existing documented maintenance exceptions for
backoff, instant, paste and serde_cbor remain unchanged; no actionable vulnerability
is suppressed.

## Validation record

Validation is in progress; the delivery report records completed commands, counts,
exact reviewed revision, workflow links and canonical artifact hashes. Logs and
machine-readable results are under ignored `observations/subnet-section/`.

Required gates: frontend setup/build/unit tests; section and ledger browser
harnesses; Rust format, locked workspace Clippy with warnings denied and workspace
tests; the full xtask ignored PocketIC integration runner; static checks; the full
four-stage security scan; source-manifest generate/verify; clean committed Docker
reproducibility and production export audits; exact-revision Behavioral Tests and
Dependency Security workflows. Test discovery is not execution evidence.

Installed hashes observed during placement verification (not new release hashes):

```text
backend  f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970
frontend 6010c28d0133fe3fb2d3941dbbe97eb3430dd760c78f95c7f75ce4eb9ae7fc77
```

## Future operator upgrade checks

Nothing is deployed by this task. After separate authorisation, follow
[deployment.md](../operations/deployment.md) for an ordinary frontend-only upgrade
using canonical artifacts, checked local mappings and `--no-create`. Do not
reinstall, upgrade the backend or alter mappings, identities or controllers.
Verify the installed frontend hash against the canonical release hash.

On the public origin confirm the floating thumbnail is gone, scroll below the hero
and find “Fully on-chain” before “The protocol”. Check live map data, country/data-
centre controls, touch, source links, mobile layout and console. Confirm the ledger
counter, operating status and subscription builder still work. This real-origin
post-upgrade check remains necessary; no replacement section is installed on
mainnet during pre-release validation.
