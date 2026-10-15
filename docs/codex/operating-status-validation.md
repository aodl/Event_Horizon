# Frontend operating-status validation

## Scope and data meaning

Work started from clean `main` at `bb3fead` on 15 October 2026. This change adds
an operating-status group beneath the Network instance selector. It leaves
backend Rust, Candid, state, configuration, pricing, subscriptions and polling
policy unchanged. No deployment, controller operation or funding action is part
of this work.

The anonymous pinned `@icp-sdk/core` client queries native management
`canister_status` and `fetch_canister_logs`, with the reviewed backend principal
as both argument and effective routing principal. Both are query calls. The
existing public status/log visibility is required; no setting is modified.
The fixed `https://icp-api.io` host, embedded root key and normal signature
verification are retained. No dependency was added.

The large balance is the current queried **total** cycles balance. The tier and
next-threshold progress come from the latest retained mode/liquid report in the
existing logs, not from a guessed balance-to-mode mapping. Both timestamps and
the reported liquid balance are displayed. This distinction matters because the
cadence uses liquid cycles and hysteresis. A dated liquid reading is not an
exact current top-up requirement, and a reported mode is not evidence of a
completed poll or delivered notification. An up-to-the-instant liquid snapshot
would require a separately approved backend interface/upgrade.

The component queries immediately, then 60 seconds after each completed
attempt, with a ten-second abortable deadline. It reuses one agent/actor on
success. Initialization/query failure retires owned transport; hidden pages,
selection changes, configuration mismatch and disposal retire pending work.
Planned IO never requests status. Unsupported/private status and missing log
reports become unavailable without disabling the pricing/memo builder.

## Validation evidence

The dedicated Node suites cover production entry-table parity, hysteretic mode
identity, BigInt arithmetic, bounded progress, highest/Reserve modes, Candid
width projection, request types, reviewed routing, trusted agent defaults,
reuse/recovery, cancellation, late results, timeout, visibility, selection,
configuration mismatch, rendering failure and builder isolation.

The browser harness passed with Chromium 153.0.8010.12, checking the built page
separately from the injected-query fixture page. Status fixture screenshots are retained under
`observations/compact-title-aspect-gate/browser/operating-{desktop,phone,narrow}.png`
for 1440×900, 390×844 and 320×568. The successful live capture
is `operating-production-live.png`; `evidence.json` separates live observations
from fixture values. Chromium emulation does not establish Firefox, Safari or
physical-device behavior. The PocketIC certified-asset test checks status
markup, exact served bundle/CSS, certification, CSP and cache revalidation.

The built page's read-only live check completed at
`2026-10-15T19:53:33.098Z`, querying backend
`eo6ei-gaaaa-aaaar-qchra-cai`. It displayed a queried total balance of
**4.864 T** and the latest log report **Economy**, **4.450 T liquid**, dated
`2026-10-15T00:00:00.481Z`. The recorded gap to Standard was **0.549 T** against
its **5.000 T** liquid-cycle entry threshold. Displayed cycle values here are
truncated to three decimals, exactly as in the UI; fixture screenshots use
separate deterministic values and are not mainnet evidence.

Results for the modified source:

| Check | Result |
| --- | --- |
| `xtask test_unit` | 237 passed: 119 Rust and 118 frontend; 106 intentionally ignored PocketIC tests excluded |
| Dedicated operating-status Node tests | 24 passed within the 118 frontend tests |
| Targeted ignored PocketIC certified-asset scenario | 1 passed (`frontend_transition_urls_are_certified_and_require_revalidation`) |
| Built-page/fixture browser harness and opt-in live query | Passed; screenshots, accessibility and request evidence retained |
| Static checks, Rust formatting, workspace Clippy | Passed |
| Frontend production Wasm export audit | Passed; no new application methods |
| Security scan | Passed; zero unfiltered findings, four existing maintenance exceptions unchanged |
| Repeated pinned frontend bundle build | Byte-identical, 226,643 bytes (7,271 bytes over baseline) |

Bundle SHA-256:
`08712205efcfbda09a45e88ac07db02b6bb0e130dbaf4b30c4b4c374b50de5eb`.
Command transcripts are ignored under `observations/operating-status-*.log`.
The complete ignored PocketIC behavioral suite was not rerun for this display
change; only the certified-asset scenario above was explicitly executed.

Reproduction commands:

```bash
npm run build:frontend
npm run test:frontend-unit
node tools/scripts/validate-ledger-counter.mjs --live
cargo run -p xtask -- test_unit
POCKET_IC_MUTE_SERVER=1 cargo test --locked -p event-horizon-pocketic \
  tests::frontend_transition_urls_are_certified_and_require_revalidation \
  -- --ignored --exact --nocapture
./tools/scripts/security-scan
./tools/scripts/source-manifest verify
```

The manifest includes the new source/tests/documentation after explicit staging.
Canonical release artifacts and mainnet deployment are separate release steps;
no canonical-build or deployment result is implied by this source validation.
