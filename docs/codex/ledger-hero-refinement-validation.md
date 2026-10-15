# Landing hero and sampled-rate refinement validation

Baseline: `50f11305ace532d3ae662a1364d8b75f5f9b76f6`, branch `main`, initially clean.
This frontend-only revision is not deployed. Historical counter validation is
retained in [the original report](ledger-counter-validation.md).

## Implementation and interpretation

One unboxed statistic follows the hero body and precedes the facts. Its right
edge aligns with fact 04. Short landscape uses an expanding first-view grid
region for statistic/facts; the original title remains below and its native
anchor is retained. Normal/portrait layouts preserve source reading order.
A viewport IntersectionObserver plus initial geometry check and coalesced
resize/scroll/reflow checks gate the compact mark. It defaults hidden, fails
closed without observation, hides for any heading intersection or above-view
heading, and removes all owned observers/listeners/frames on disposal.

The browser still queries only the reviewed selected observed ledger, directly:
ICP `query_blocks({start:0n,length:0n})` yields historical `chain_length`, including
archived/pre-service history and all ICP transaction types. Generic ICRC-3 uses
`icrc3_get_blocks([{start:0n,length:0n}])` / `log_length` and records terminology.
No backend processing count or instantaneous/certified tip is inferred.

Rate = accepted BigInt count difference / actual monotonic receipt interval.
Keep 60 seconds / at most 32 samples, require a 15-second oldest-to-newest span,
round to one decimal with `<0.1` for positive subprecision growth and true zero
for unchanged qualifying samples. No additional queries/timers. Failure, lower
reply, stale data, pause/hidden, selection/mismatch and disposal invalidate rate
history. Reduced motion retains sampling, direct totals and static artwork.
Polling stays immediate, one 2-second warm-up, then 15 seconds after completion;
deadline 10 seconds, retries 30/60/120 seconds, delayed threshold 45 seconds.

## Evidence and release gates

New evidence is ignored under `observations/ledger-counter-refinement/`; it
records the exact final revision used for clean canonical builds. The tracked
source manifest includes this report. Generated hashes/logs are kept outside
tracked build inputs so final build evidence cannot change those inputs.

The browser matrix includes 1440×900, 1280×800, 1024×768, 1000×800, 844×390,
390×844, 360×640, 320×568 and 200% text. The 1000×800 and 1024×768 cases test
layouts where the old CSS-only condition displayed both headings. Tests check
actual intersecting rectangles, positive shortcut visibility in short landscape,
dynamic scrolling/resizing/anchor focus, unboxed styling, fourth-fact alignment,
rate pending/nonzero/zero/tiny values, pause/delay/planned and huge generic totals.

Current primary references consulted: [MDN Intersection Observer](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API),
[ICP certified data](https://docs.internetcomputer.org/concepts/certified-data/),
and [DFINITY certified-variable guidance](https://github.com/dfinity/icskills/tree/main/skills/certified-variables).
The pinned SDK and certified HTTP serving remain unchanged: static-asset
certification does not certify runtime ledger samples.

Limitations: Chromium emulation is not Safari/Firefox or physical-device proof.
No live ICRC-3 deployment is invented; IO remains planned. Browser fixture samples
are deterministic and separate from production transport. No mainnet mutation,
deployment, controller operation, transfer, backend change or remote push is
authorized/performed in this refinement.

## Local results before clean canonical validation

- Frontend deterministic tests: **94 passed**, zero failed/skipped. New tests
  separately cover compact-heading lifecycle/geometry, sampled-rate arithmetic,
  history/window limits, retries/deadlines, precision and page integration.
- `cargo fmt --all -- --check`, locked workspace/all-target Clippy with warnings
  denied, and static checks passed. No security policy was weakened.
- Security scan passed with zero unfiltered findings. The four existing
  maintenance-only exceptions remain separately identified (backoff, instant,
  paste, serde_cbor); no dependency or exception changes were made.
- Bundle: **219,447 bytes**, versus **217,076** at baseline (**+2,371 bytes**).
  Two locked builds produced identical SHA-256
  `23f20981dca341f01209d7bde0a3f283a52eeb573de9ea32b0b346d230b8d725`.
- CSS/JS suffixes remain `v=9`/`v=3`: existing mandatory revalidation and
  query-suffixed certified routing remain tested; no immutable-cache policy is
  introduced. PocketIC asserts the rate markup and exact served asset bytes.

Canonical hash equality, final source revision, fresh complete PocketIC totals,
browser screenshots and gate exit statuses are recorded in the ignored final
release evidence (`release-validation.json`) and completion report. This source
report makes no claim about a build or test that has not yet completed.

## Read-only live ICP evidence

At 2026-10-15T07:05:15.664Z the pinned agent accepted **39,159,927**; at
07:05:33.334Z it accepted **39,159,928** from canonical ledger
`ryjl3-tyaaa-aaaaa-aaaba-cai`. The actual monotonic receipt span was
17,670.620369 ms, yielding **<0.1 tx/s**. This opt-in smoke made two zero-length
`query_blocks` calls and one initial signature-key `read_state`, reusing the
session. The embedded mainnet root key and normal verification were retained.
It submitted no transactions and is separate from deterministic visual fixtures.
Full request/timestamp evidence is `live-recent-rate.json` in the ignored output.

## Changed files

- `MANIFEST.sha256`, `SPEC.md`
- `canisters/frontend/README.md`
- `canisters/frontend/public/app.js`, `app.bundle.js`, `compact-heading.js`,
  `index.html`, `ledger-counter.js`, `styles.css`
- `canisters/frontend/test/compact-heading.test.mjs`, `generic.test.mjs`,
  `helpers.mjs`, `ledger-counter-integration.test.mjs`, `ledger-rate.test.mjs`
- `docs/README.md`, `docs/development/testing.md`, this validation report
- `tests/pocketic/src/lib.rs` (certified frontend markup assertions only)
- `tools/scripts/validate-ledger-counter.mjs`, `tools/static-check.py`

No production dependency, ledger client, instance identity, backend Rust/Candid,
frontend Rust/Candid, trust setting or backend hash pin changed. No files removed.

## Browser result

Pinned Playwright / Chromium **153.0.8010.12** passed the actual built page and
separate injected-query fixture. Both the stable geometric assertions and a
frame observer during dynamic scroll/resize/disclosure tests found no simultaneous
visible hero brands. The positive 844×390 shortcut case, native keyboard anchor,
focus retirement, no-JavaScript hiding and missing-observer fail-closed behavior
passed. Every requested viewport has pending and populated-rate captures:

| Viewport | Screenshot basename under `observations/ledger-counter-refinement/browser/` |
| --- | --- |
| 1440×900 | `desktop.png`, `desktop-rate.png` |
| 1280×800 | `compact-wide.png`, `compact-wide-rate.png` |
| 1024×768 | `compact.png`, `compact-rate.png` |
| 1000×800 (old duplicate-title case) | `regression.png`, `regression-rate.png` |
| 844×390 | `landscape.png`, `landscape-rate.png` |
| 390×844 | `phone.png`, `phone-rate.png` |
| 360×640 | `small-phone.png`, `small-phone-rate.png` |
| 320×568 | `narrow.png`, `narrow-rate.png` |
| 200% text | `regression-text-zoom.png`, `zoomed.png` |

Additional captures: `nonzero-rate.png`, `zero-rate.png`, `small-rate.png`,
`paused.png`, `delayed.png`, `unavailable.png`, `production-planned.png`,
`long-value.png`, `reduced-motion.png`; accessibility snapshot `accessibility.yml`
and request/console/geometry record `evidence.json`. Numeric rate is absent until
a qualifying span, on pause/delay/planned states and after recovery resets.
All viewport checks found no horizontal overflow, no statistic panel border/fill,
right-edge alignment with fact 04 and no body/statistic overlap. Visual inspection
covered desktop/phone and short landscape over the approved artwork; focus and
44-pixel control targets remain available. Actual SVG timeline/CSS pausing,
reduced-motion asset replacement, quiet exact sampled total once, simulated DOM
visibility return, pricing isolation and timer/frame cleanup passed. No page
exceptions, CSP errors or unexpected request hosts were observed.

A first extended screenshot pass hit a 15-second capture timeout while Rust was
compiling. Raising only the browser evidence capture budget to 60 seconds and
limiting frame monitoring to the dynamic regression section produced a complete
pass; no production code, verification setting or assertion was weakened for it.
Physical tab hiding remains simulated through document visibility events; desktop
Chromium emulation does not establish real-device performance or other engines.

Deterministic suite breakdown: general/builder/artwork **30**, adapter/wire/
transport **17**, counter controller/precision **27**, page integration **9**,
sampled rate **8**, compact heading **3**; **94 total, all passed**.
