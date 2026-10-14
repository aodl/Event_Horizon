# Landing ledger-counter validation

## Scope and revision

Baseline: clean `28788a57a410236cc65aeea0ef401fb40da6d44e` on 14 October
2026. This frontend-only change adds a selected-ledger history total and shared
live-effects control. Backend logic, stable state, settings, Candid, immutable
instance identities, pricing, funding, subscriptions, cadence, scheduler and
reviewed backend hash are unchanged. No deployment, transaction, top-up,
reinstall, controller change or other mainnet mutation is part of this work.

The final source revision is the commit containing this report, resolved with
`git log -1 --format=%H -- docs/codex/ledger-counter-validation.md`. Canonical
builds run only after that clean commit exists. Their exact revision, hashes and
outcomes are retained in ignored `observations/ledger-counter/release-validation.json`
and reported with the release, avoiding a self-referential commit hash inside a
tracked build input. Generated evidence is not shipped as frontend content.

## Implementation and interpretation

- ICP explicitly selects `icp-chain-length` on the existing observed-ledger
  principal. IO remains planned, `ledgerCountSource: null`, with no request.
  Reviewed generic entries may explicitly opt into `icrc3-log-length`; unknown
  modes are unavailable. No symbol-based adapter discovery occurs.
- Canonical requests use one `query_blocks` record with `nat64` zero start and
  length; generic requests use one `icrc3_get_blocks` vector with `nat` zero
  start and length. Counts come directly from `chain_length` / `log_length`.
  There is no index adjustment, transaction-body request or archive traversal.
- The headline is **ICP ledger transactions**, including all history and all
  transaction kinds. Generic fixtures use **Reviewed fixture ledger records**.
  The explanation disclaims Event Horizon processing, notification, catch-up and
  polling interpretations. The backend's prospective cursor is not a processing
  counter, particularly because proved archive prefixes can advance it.
- First acceptance renders directly. Exact BigInt linear interpolation is
  bounded by accepted totals, including numbers beyond `2^53` and `2^64`.
  Equal replies refresh time without extending animation; quiet data holds.
  Accepted data, displayed integer, interpolation target and receipt time are
  distinct. No persistence or fabricated historical count-up exists.
- Defaults: immediate first sample; one 2-second warm-up; 15-second steady
  delay after completion; 15-second interpolation; 10-second whole-attempt
  deadline; 30/60/120-second capped failure backoff; 45-second delayed threshold;
  three lower replies since acceptance trigger inconsistent-data indication.
- Lower replies preserve high-water and accepted timestamp and do not clear
  failures. Later equal or higher data recovers. Resuming or recovering delayed
  data displays a current baseline directly, retaining same-ledger monotonicity.
- The separate lazy agent/actor session reuses normal mainnet verification and
  embedded root key. `retryTimes: 0` removes transport retry chains. Owned fetch
  is aborted on failure/timeout/retirement, and the entire retired session is
  permanently invalidated. Initialization failures are evicted. Generation and
  attempt ownership checks protect success, rejection, timers, frames and
  `finally`, without cancelling pricing transport.
- Positive configuration mismatch is a typed result from existing verification;
  it retires the statistic. Pricing transport failure alone does not. Counter
  failure does not change verified pricing or builder behavior. Switching clears
  both the prior statistic and existing memo/funding instruction immediately.
- Hidden/manual pause freezes polling and numeric updates. Dynamic reduced motion
  shows direct samples and static artwork; removing it cannot override pause.
  Artwork retains blur/focus behavior and synchronizes repeated object loads.
  The control calls SVG timeline pause and pauses CSS animation. Numeric and
  lifecycle timers, frames, transport and owned listeners have explicit cleanup.
- Safe text sinks render values and status. The visual numeral is excluded from
  the accessibility tree; the latest exact sample is exposed once without a live
  region. The explanation and native pause button work by keyboard/touch. Long
  generic totals wrap without truncation. The compact layout uses opening artwork
  space; the title, approved artwork, glow, placeholder, fade and facts remain.
  The body's minimum width and title wrapping also permit narrow text zoom.

## Deterministic evidence

`npm run test:frontend-unit` passes **82 tests**: 30 existing general/builder/
pricing/artwork tests, 17 client/wire/transport tests, 8 page integration tests,
and 27 controller/precision/lifecycle tests. No real-time sleeps or mainnet calls
are part of these suites. The property case uses a fixed seed and 1,000 generated
large-integer interpolation paths. Coverage includes genuine zero, malformed and
wrong-scalar replies, equal samples mid-animation, retargeting, exact arrival,
clock reversal, large frame gaps, all failure backoff tiers, stale transitions
without animation, contradictory response streaks, selection reversal, shared
ledger selections, planned entries, missing elements, typed mismatch, pricing
isolation, manual-pause precedence, reduced motion, listener cleanup, and late
resolve/reject after retirement. Composed controller/client tests verify actual
owned fetch cancellation at the initialization/query deadline before retry.

Independent full wire fixtures in `ledger-wire.mjs` are encoded using the pinned
`@icp-sdk/core` 6.1.0 Candid implementation and decoded using the production
projection. They include complete ICP block/operation types, empty and populated
block vectors, certificates, archive callback functions, recursive ICRC values,
recursive archive replies and extra fields. This proves record-width projection
compatibility without fabricated null/empty replacement types. No archive callback
executes. Retained upstream interface references, consulted on 14 October 2026:

- [ICP ledger.did](https://github.com/dfinity/ic/blob/8ac830931a8b2ca633064197322c2ae535650060/rs/ledger_suite/icp/ledger.did)
- [ICRC-3](https://github.com/dfinity/ICRC-1/blob/5d670e54d9a58fbf472bf0a25f33743d60cfd0e6/standards/ICRC-3/README.md)
- [DFINITY skills](https://github.com/dfinity/icskills/tree/030377fa970084c25aae33c438004d2449e61b48/skills):
  `icrc-ledger`, `certified-variables`, and static-site context. The existing
  custom certified Rust frontend remains in use.

## Browser evidence

The pinned development-only Playwright harness runs Chromium 153.0.8010.12.
The unmodified built page is served with the production CSP/cache policies for
unavailable/planned states, keyboard-accessible pause, repeated SVG replacement,
and actual timeline/CSS pause checks. Deliberately blocked ledger requests are
recorded as failures, separate from the pricing client's own retries.

A separately compiled test-only entry runs the production page integration with
injected queries/clocks, not a signature-verification bypass. Screenshots cover
1440×900, 1024×768, 390×844, 360×640 and 320×568. The statistic fits the initial
viewport at each ordinary size. Additional captures cover interpolation, delayed
state, a 60-digit generic total, planned state and 200% text sizing at narrow
width. The numeral groups exactly and wraps; horizontal-overflow assertions,
keyboard explanation/control traversal, accessible exact-sample uniqueness and
component cleanup are checked. Visibility return is simulated through DOM events
and a test-only visibility getter, separately from real SVG motion checks.
Request and console checks reject unexpected hosts, page exceptions and CSP
violations. Images and `accessibility.yml` are under
`observations/ledger-counter/browser/`; `evidence.json` identifies fixture and
production/live results separately.

Only Chromium and emulated dimensions have been exercised. Firefox, Safari,
physical touch devices and low-powered-device performance are not established.
Headless focus/visibility simulation is not proof of every OS/browser suspension
behavior. No continuous background polling, online/offline subsystem or
intersection observer was added.

## Read-only live evidence

The bounded opt-in Node command used the actual pinned client, normal signature
and certificate verification and configured `https://icp-api.io` host. Principal:
`ryjl3-tyaaa-aaaaa-aaaba-cai`; method: `query_blocks`; argument:
`{ start: 0n, length: 0n }`, both `nat64`.

| Accepted response UTC | Reported total |
| --- | ---: |
| 2026-10-14T22:17:04.041Z | 39,151,279 |
| 2026-10-14T22:17:06.388Z | 39,151,280 |

The first attempt made one query and one `read_state` for signature keys; the
second reused the session and made one query. Requests were sequential logical
attempts. This demonstrates nonzero network cost and actual verified ICP adapter
operation, not guaranteed future freshness or activity. `live-smoke.json` retains
request timestamps and results. The browser harness's optional `--live` pass
records its independently accepted samples in `browser/evidence.json`.
No reviewed live ICRC-3 ledger was queried. Generic support is wire/fixture and
DOM integration evidence; IO's launch status and principal remain unchanged.

Runtime ledger samples are **not** covered by the frontend static asset
certificate tree, and no certified-tip proof is claimed. SDK verification does
not prove the answering replica held the newest possible state.

## Repository and release gates

The complete full-suite, source/security and clean-commit canonical commands are
recorded in ignored logs and the final release result. The PocketIC frontend
scenario explicitly checks the new markup and exact served built bundle/CSS,
including query suffixes, certification, CSP and revalidation. Production export
audits must still find exactly backend `get_instance`, `get_pricing`, and frontend
`http_request` application queries.

The production dependency stays `@icp-sdk/core` 6.1.0 and esbuild stays 0.28.2.
Playwright 1.61.1 plus its matching core is a pinned development-only browser
harness dependency compatible with the canonical Node 18 toolchain; no browser
binary enters production assets. No Rust dependency or security policy changes
are required. Security scanning retains the four separately identified existing
maintenance exceptions without adding exceptions. The rebuilt frontend bundle
is compared with a second locked rebuild. The baseline bundle was 208,989 bytes;
final size and artifact hashes are recorded with the release evidence.

Backend canonical SHA-256 must remain:
`e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca`.
The prior frontend hash was
`8732b798be30ecdead974eae4c15b61ae39f2dd6a9d56bbf0be568e06b5e4e01`;
the new frontend is expected to differ because its embedded assets change.
The backend hash registry is not repinned. Canonical reproducibility compares
two independent clean builds of the final source revision. Only the frontend is
a deployment candidate, and deployment remains a separate operator action.


## Final local results and changed files

The final implementation's full runner completed **307 passed tests across 111
suites**: 119 Rust unit tests, 82 frontend Node tests and 106 fresh PocketIC
scenarios, with no failures. Both certified-frontend scenarios passed with the
final embedded assets. The first complete run's outer command session reported
termination code 143 despite the complete success summary; a repeat with durable
process-exit capture supplies the final gate status in `release-validation.json`.
The interrupted earlier development run is excluded from release counts.

Formatting, Clippy, static validation, source-manifest verification and both diff
checks passed. Dependency security passed cargo-audit, cargo-deny, production npm
audit and OSV with zero unfiltered findings; the four existing maintenance
exceptions remain separately identifiable and unchanged. Browser validation on the
final controller passed; its two production-page samples both reported
**39,151,415**, with the second accepted at **2026-10-14T22:32:46.273Z**. An equal
sample is a valid freshness check and produces no fabricated movement.

The final bundle is **217,076 bytes**, up **8,087 bytes (3.87%)** from baseline.
Its SHA-256 is
`618dd1bc0a0fc775d5dc355948c70542e67014f2ff0906ef03a0f482a40bebe4`;
a repeated locked rebuild is byte-identical. No files were removed. Exact changed
files (all other source is preserved):

- `MANIFEST.sha256`
- `SPEC.md`
- `canisters/frontend/README.md`
- `canisters/frontend/public/app.bundle.js`
- `canisters/frontend/public/app.js`
- `canisters/frontend/public/index.html`
- `canisters/frontend/public/instance-view.js`
- `canisters/frontend/public/instances.js`
- `canisters/frontend/public/ledger-counter-client.js`
- `canisters/frontend/public/ledger-counter.js`
- `canisters/frontend/public/styles.css`
- `canisters/frontend/test/counter-clock.mjs`
- `canisters/frontend/test/generic.test.mjs`
- `canisters/frontend/test/helpers.mjs`
- `canisters/frontend/test/ledger-counter-client.test.mjs`
- `canisters/frontend/test/ledger-counter-integration.test.mjs`
- `canisters/frontend/test/ledger-counter.test.mjs`
- `canisters/frontend/test/ledger-wire.mjs`
- `docs/README.md`
- `docs/codex/ledger-counter-validation.md`
- `docs/development/testing.md`
- `docs/operations/deployment.md`
- `package-lock.json`
- `package.json`
- `tests/pocketic/src/lib.rs`
- `tools/scripts/check-ledger-count.mjs`
- `tools/scripts/validate-ledger-counter.mjs`
- `tools/static-check.py`
