# Hero fact icons and linked explanations

## Scope

Baseline: clean `main` at `c2b8007b8ad1a8ee1dcea4de36ab150a7b20d84d`,
17 October 2026. This is a frontend presentation/content change. The four hero
facts become native icon-and-label links to separate editorial sections between
Principles and additional instances. Backend Rust, Candid, stable state, prices,
cadence, settings, identities and the reviewed backend registry pin are unchanged.
No deployment, mainnet query, funding or controller action is part of this task.

| Label | Target | Local asset |
| --- | --- | --- |
| Open Source & Verifiable | `#open-source` | `/github-mark-white.svg` |
| Designed for Immutability | `#immutability` | Existing `/favicon.ico?v=1` |
| Productive Endowment Funding | `#endowment-funding` | `/jupiter-faucet.ico` |
| Powered by Internet Computer Protocol | `#internet-computer` | `/icp-infinity.svg` |

The text remains the sole accessible link name; each image has an empty
alternative. Native fragment navigation, focusable labelled sections and scroll
margins support keyboard use, direct links and history. The existing
reduced-motion rule supplies immediate scrolling. Wide desktop keeps four
columns; tablet uses two and narrow mobile one, with 44px minimum link height.
Brand marks have no colour filters, distortion, animation or background panel.
The existing art-first compact hero, compact-heading observer and independent
ledger-counter region are retained.

The sections explain reproducible Wasm and independent live identity/configuration/
controller checks, intended controller-removal finalisation, a separately
governable frontend, delayed evidence-based admission, productive principal and
pooled operating receipts, and ICP hosting with best-effort notifications.
They make no claim that controller removal or the planned protocol seed has
already happened, or that perpetual operation/delivery is guaranteed.

## Asset provenance

All files were obtained on 17 October 2026. The three new files are unchanged
copies; the Event Horizon icon is reused rather than duplicated. The associated
trademarks remain their owners' property.

- **GitHub:** white Invertocat, `GitHub Logos/SVG/GitHub_Invertocat_White.svg`
  from the [official GitHub logo archive](https://brand.github.com/GitHub_Logos.zip),
  linked by the [GitHub Brand Toolkit](https://brand.github.com/foundations/logo).
  Archive SHA-256: `e2a67d6cc51d990a52c46c1cf6bcab688db4830982174bca50e0be7a5c2f3194`.
  Local SVG: 1,559 bytes,
  `ccd84c89b1056345608fc3489357f8acc7397e49a3cdc2d418b6c8016911d47b`.
- **Event Horizon:** the existing 16,732-byte multi-size ICO,
  `56912bdc9d6c862359db56a018b7242dcefe634f28f0e1099c170bccb63ed36b`.
- **Jupiter Faucet:** its [actual frontend favicon](https://github.com/aodl/JUPITER_FAUCET_SUITE/blob/e0c0c4df247e84ee42f23f568a56e165bf522152/canisters/frontend/public/favicon.ico),
  pinned to source revision `e0c0c4df247e84ee42f23f568a56e165bf522152`.
  Local ICO: 2,421 bytes,
  `37195f946930cfa8349372f451fc5ba4910b577ac6fe133337e93169be48b6d6`.
- **Internet Computer:** the [current official site's full-colour infinity favicon](https://internetcomputer.org/favicon.svg),
  copied unchanged as `icp-infinity.svg`. Local SVG: 2,128 bytes,
  `64f4576c27dd0ff9815ddd63e9eeb589f1e80d3c81bb5d6baf3c3f537cc46934`.
  It preserves the official gradients, paths, aspect ratio and transparent clear
  space. Its 44px square image box displays the wide mark at about 39×20px;
  other icon boxes are 24px. The previously published Frontify guide URL
  (`https://dfinity.frontify.com/d/pD7yZhsmpqos/internet-computer`) returned 404;
  the current official-site artwork was used without alteration.

Only 6,108 bytes of new artwork are added. No icon library, font, remote image
request, runtime dependency or serving-code change is introduced. Existing SVG,
PNG and ICO certification/media rules apply. CSS URL moves to `styles.css?v=10`;
the JS bundle remains byte-identical because the browser clients are unchanged.

Current DFINITY canister-security guidance was consulted; retained text SHA-256:
`a22daeb650cdc37ef6b3dc4a1f5cdcc35842a795459ac2953eff2412bcc4a022`.
Its advice does not authorise changing this repository's pinned SDK, embedded
trust configuration or immutable backend design.

## Validation and release evidence

The dedicated Node tests check exact fact labels/order, local decorative icons,
unique labelled targets/placement, source-byte hashes, inactive SVG content and
separate external resources. The browser harness additionally checks actual
image decoding, accessible link/heading names, keyboard focus and tab order,
native URL/history/direct fragments, reduced-motion scrolling, responsive
wrapping and existing hero/counter regressions. Chromium emulation is not a
Firefox, Safari or physical-device performance claim.

Frontend validation passed: **125 Node tests** and the browser harness on
Chromium **153.0.8010.12**, including all eight icon-link viewport cases and
200% text. The existing compact title, art placement, lower-right statistic,
pause/reduced-motion and counter recovery checks also passed. Static checks,
Rust formatting, workspace Clippy and the security scan passed; the scan retains
the same four separately documented maintenance exceptions and no unfiltered
findings. The full repository/PocketIC suite and clean-commit canonical results
are recorded separately in the retained command logs and final release evidence.

Run:

```bash
npm run build:frontend
npm run test:frontend-unit
node tools/scripts/validate-ledger-counter.mjs
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all
./tools/scripts/security-scan
./tools/scripts/source-manifest verify
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
(cd release-artifacts && sha256sum -c release-artifacts.sha256)
```

Browser screenshots, accessibility data, command logs and final canonical
results are retained in ignored `observations/hero-facts/`; canonical build
commands run only after the reviewed source is committed and clean. Their
resolved source revision and final frontend hash belong to the resulting
artifact evidence and completion report, rather than an earlier build.
The required canonical backend comparison is
`e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca`.
