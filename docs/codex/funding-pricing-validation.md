# Funding documentation and 1,000 ICP global pricing validation

## Identity and scope

- Starting Event Horizon revision: `580e37863027163d8bf578ae91ac6be353f582fe`.
- Pricing and documentation implementation revision:
  `24cf78f8b4a2ea79426f9676f6eb04a2e1283034`.
- Final artifact-source revision:
  `54d8b4ba99e6392cfbfa3a5d064ce8f57f8378d7`.
- The evidence-only revision containing this report and the regenerated source
  manifest is identified in the handoff because a commit cannot contain its own
  identifier.
- No mainnet deployment, install, reinstall, upgrade, ICP or cycles transfer,
  alias publication, controller change, or other mainnet mutation occurred.

The production behavior change is deliberately limited to
`GLOBAL_BASE_ICP: 100 -> 1_000`. Account remains 10 ICP and range remains 20
ICP. No production method, install argument, stable-memory ID, stored `Price`
field, cadence, matching rule, admission rule, funding state machine, or
surplus rule changed.

## External protocol references

Jupiter Faucet Suite was inspected at
`a484ae10eaee9b47b64657d1f7e5d70eec8e4fe7`. Its source-controlled memo policy
accepts a nonempty `X.<suffix>` ASCII memo up to 32 bytes, routes raw ICP to the
canonical Event Horizon destination, and preserves the suffix. The reviewed
production configuration uses a 604,800-second stake-recognition delay. The
same source establishes the productive-stake, Disburser, Faucet payout, later
round, and optional poke-acceleration behavior described in the current docs.
Faucet source owns the alias policy; Historian supplies route history and
verification evidence.

Current DFINITY upgrade/reinstall guidance was reviewed at ICP portal revision
`ed742236de3b54ed56161f188913536259941a12`: an upgrade preserves stable state,
whereas reinstall resets it. No external repository was modified.

`X.protocol-seed` is 15 bytes. At the reviewed Jupiter revision it would route
the exact outgoing suffix `protocol-seed`; Event Horizon's production parser
rejects that suffix as `InvalidPrincipal`, proven by
`memo::tests::protocol_seed_route_cannot_declare_a_subscription`. This is
source-level compatibility only, not a reservation, publication, deployed
configuration check, or authorization to transfer funds. Operators must
reconfirm deployed Jupiter policy before using it.

## Pricing implementation and arithmetic

For retained floor `F` and current eligible ICP/XDR observation `C`, production
continues to use checked integer arithmetic and independent upward rounding:

```text
account_icp = ceil(10 × F / C)
range_icp   = ceil(20 × F / C)
global_icp  = ceil(1000 × F / C)
```

The persisted `Price` remains `{ account_icp, global_icp }`. Public and
admission range prices continue to derive exactly from the independent 20 ICP
basis using the existing constant-derived divisor:

```text
ceil(20F/C) = ceil(ceil(1000F/C) / 50)
```

The unit matrix proves 10/20/1,000, 5/10/500, 2/4/200, 7/13/625, and
3/5/210. A one-million-pair integer sweep plus large-value cases proves the
derived range equals the direct 20-basis formula. The overflow case remains an
error rather than wrapping.

The pricing test was changed before the constant. The focused native rerun
failed at `F=10, C=10`, reporting actual global 100 versus expected 1,000. The
same test passed after the production constant changed.

## Admission and stable-state evidence

PocketIC establishes the exact global boundary:

- `GLOBAL_BASE_E8S - 1` (999.99999999 ICP) is insufficient.
- `GLOBAL_BASE_E8S` (1,000.00000000 ICP) is sufficient.
- Exact Historian route totals of 600 ICP and 999 ICP are insufficient; 1,000
  ICP, represented by 600 + 400, is sufficient.
- Global admission remains permanent, requires an authorised Faucet payout and
  exact Historian evidence, and preserves global/specific poke precedence and
  poll-level coalescing.
- Pricing initialization and the monthly freeze/activation path produce
  10/20/1,000 initially and 5/10/500 at the tested rate change.

An actual committed pre-policy debug Wasm fixture was installed in PocketIC.
It established nonzero cursors, subscriptions, and 10/20/100 pricing. An
ordinary upgrade to the current Wasm retained its state and caught up its
reader, but also retained global 100, which the new divisor exposed as range 2.
That test deliberately proves an ordinary policy-crossing upgrade is unsafe.

The selected prelaunch strategy is therefore an exceptional controlled
reinstall, not migration code and not the normal release procedure. A second
test reinstalls the current Wasm over that same old fixture and proves that
pricing observations, cursors, subscriptions, and pending funding state reset;
immutable constructor configuration is re-supplied; pricing starts
uninitialized at 10/20/1,000; and the readers bootstrap prospectively without
replaying historical Faucet payouts.

Reinstall is permissible only after an operator freshly verifies that the
deployed canister has no subscriptions, pending financial actions, or other
obligations that must survive, and separately approves the destructive reset.
If such state exists, this work does not declare reinstall safe and no general
migration mechanism was added. Subsequent ordinary releases remain
argument-free, state-preserving upgrades. Reinstall must never be used merely
to clear an ordinary Ledger backlog.

## Funding and documentation result

Current documentation now treats
`docs/architecture/economics-pricing.md` as the authoritative explanatory
reference. It distinguishes:

1. the planned 3,000 ICP productive protocol endowment for recurring baseline
   payouts;
2. account, range, and global subscriber endowments that add productive backing
   and enable permanent admission; and
3. an independently supplied liquid-cycles bootstrap needed while payouts are
   pending and potentially to reach the 100 T Continuous entry threshold (60 T
   exit remains unchanged).

It explains that productive principal enters Jupiter's configured NNS neuron,
not Event Horizon's liquid balance. Variable maturity-derived raw ICP payouts,
after Historian-backed admission checks, enter Event Horizon's common reserve
and are converted to cycles. Endowment principal and distributed rewards are
separate accounting concepts; no yield, cadence, solvency, or uninterrupted
service is guaranteed. Additional funding creates neither polling priority nor
a subscriber-specific cycles balance.

The README and specification carry the corrected formula and permanent-funding
summary. Architecture documents cover the funding flow, pooled sustainability,
stable-price boundary, trust split, and limitations. Subscriber and instance
guides explain patron funding, seven-day recognition delay, cumulative exact
routes, third-party patrons, permanent admission, and ICP-funded service for a
separately observed token Ledger. Operations documents distinguish the
exceptional reset from normal upgrades, prescribe prospective reader bootstrap
before accepting new endowments, separate the planned 3,000 ICP endowment from
liquid cycles, and add read-only sustainability/controller-removal checks.
Canister READMEs and the docs index were aligned.

The frontend still uses authoritative `get_pricing` values and performs no
local price calculation. Copy now describes minimum qualifying productive
endowments and separates principal from recurring operating payouts. Tests
cover independently rounded 7/13/625 values, initial 10/20/1,000 values, and a
higher frozen 11/21/1,101 response. The generated bundle was rebuilt from
`app.js`.

## Tests and gates

All commands ran from `/home/codexdev/src/Event_Horizon` unless noted.

- `cargo test --locked -p event-horizon --lib
  pricing::tests::formula_examples_and_independent_rounding -- --exact`:
  failed before the correction for the expected old-100/new-1,000 mismatch;
  passed afterward.
- Focused pricing unit suite: 11 passed.
- Focused protocol-seed memo test: 1 passed.
- Focused PocketIC tests passed:
  `global_admission_uses_exact_cumulative_historian_total`,
  `global_admission_and_poke_precedence_are_enforced`,
  `pricing_initialization_failure_recovery_freeze_and_activation`,
  `baseline_reader_failure_upgrades_in_place_and_catches_up_from_retained_cursor`,
  and `prepolicy_state_is_replaced_by_the_controlled_pricing_reinstall`.
- `cargo run --locked -p xtask -- test_unit`: 5 suites and 113 tests passed
  (88 Rust plus 25 frontend), zero failures.
- `POCKET_IC_MUTE_SERVER=1 cargo run --locked -p xtask -- test_all`: 111
  suites and 219 tests passed (88 ordinary Rust, 25 frontend, and 106 isolated
  PocketIC), zero failures.
- `npm run test:frontend-unit`: 25 passed, zero failures.
- `npm run build:frontend`: passed; generated bundle is 203.7 kB.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `./tools/scripts/security-scan`: passed. npm audit found zero
  vulnerabilities and OSV found no issues after the repository's four reviewed
  unmaintained-transitive filters (`backoff`, `instant`, `paste`, and
  `serde_cbor`).
- `./tools/scripts/source-manifest generate` and `verify`: passed for the
  artifact-source revision and are rerun after this evidence report.
- `npm run verify:reproducible-artifacts`: passed; two independent no-cache
  builds produced byte-identical backend and frontend Wasm.
- `./tools/scripts/docker-build`: passed from a git archive of
  `54d8b4ba99e6392cfbfa3a5d064ce8f57f8378d7`; artifact manifest verification
  passed.
- `tools/audit-wasm.py`: backend export audit passed with 10 exports and
  frontend export audit passed with 7. Production Candid/application surfaces
  remain unchanged and no debug method appears in production Wasm.

## Canonical artifacts and pin agreement

Canonical outputs from artifact-source revision
`54d8b4ba99e6392cfbfa3a5d064ce8f57f8378d7`:

- backend Wasm SHA-256:
  `e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca`;
- frontend Wasm SHA-256:
  `bb028354e35b22453a6838aeaa8ca45a69859544ba330c17a0fb2e6e0eb31a9e`;
- `build-info.json` SHA-256:
  `d46252b76372544251ca7485f95b9caca85c1e251d3854b2422a85757a725816`;
- artifact manifest SHA-256:
  `97d59023ffe3f93470a15441850ed5d352d3b88c559effb5407319573a10712d`;
- artifact-source `MANIFEST.sha256` SHA-256:
  `6cc894d2cf20887ee0cb765d249c2a518ee354438583414d5e1ab1ba59faf03d`;
- backend Candid SHA-256:
  `ff3b43e685ac8386cea58ce5403ee29e14200af73509c21bffbb93bcaf55ed42`;
- frontend Candid SHA-256:
  `f42355be5adc510d4edf19c260634cd53b65ef50f400fa7b7ba3c1c362487def`.

The canonical ICP entry in `canisters/frontend/public/instances.js` pins the
exact backend Wasm hash above, and the static contract check enforces it. This
registry is a reviewed expected value; the browser does not query the management
canister for a live module hash. Operators must compare the installed module
hash separately before acceptance.

## Remaining operator decisions

- Reverify current production subscriptions, pending financial state, balances,
  and obligations before deciding whether the exceptional reinstall is safe.
- Reconfirm the deployed Jupiter alias policy and configuration before
  approving any 3,000 ICP transfer or the candidate `X.protocol-seed` memo.
- Supply and observe adequate liquid cycles separately from the productive
  protocol endowment.
- Observe actual Faucet receipts, conversions, poll/poke rates, cycle burn, and
  reserves; the documented numerical scenarios are hypothetical, not proof of
  sustainability.

No mainnet action was performed by this validation.
