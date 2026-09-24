# Range subscriptions validation

## Identity and scope

- Starting commit: `c3cb4b41f390cbe7ce81e8ef6976ff2527ee5161`.
- Branch: `codex/range-subscriptions`.
- Final commit: the handoff `HEAD` containing this report and the regenerated root manifest; its exact SHA is reported with the archive because a Git commit cannot contain its own identifier.
- Commits added before the final evidence commit: `7dd9537` grammar/domain parsing, `5a77096` admission expansion, `836ee66` pricing read model, `b6e44c1` frontend, and `7481208` PocketIC/migration coverage.
- No mainnet deployment, Jupiter Faucet `X` alias publication, or controller change was performed.

## Final grammar and validation

```text
X.<canister>
X.<canister>.<subaccount>
X.<canister>.<subaccount>:<amount>
X.<canister>.<start>-<end>
X.<canister>.<start>-<end>:<amount>
```

Range endpoints are inclusive canonical decimal integers satisfying `0 <= start < end <= 255`. Reverse, degenerate, malformed, signed, leading-zero, and out-of-bounds forms are rejected without repair. Optional thresholds retain the existing decimal grammar, `0.01 ICP` floor, two-fractional-digit limit, and inclusive `amount >= threshold` match. The complete `X.` memo remains limited to 32 ASCII bytes; the frontend continuously displays the exact draft byte count and never drops or rewrites fields to fit.

## Pricing and stable state

For retained floor `F` and current rate `C`:

```text
account = ceil(10F/C)
range   = ceil(20F/C)
global  = ceil(100F/C)
```

The durable `Price { account_icp, global_icp }` encoding did not change. Public current/frozen values and admission derive `range_icp = ceil(global_icp / 5)` using quotient/remainder ceiling division. A property test covers every `F,C` pair in `1..=1000` plus large integer boundaries and proves equality with direct `ceil(20F/C)`. The `2.1/4.2/21` boundary produces `3/5/21`, not `3/6/21`.

No stable-memory ID, key, or value encoding changed. Admitted ranges expand directly into memory ID 1, the existing watched-account map. Each account retains one record and overlaps merge to the lowest admitted threshold. Since declarations are permanent, effective thresholds only stay equal or become less restrictive. No persistent range registry or runtime range matcher was added.

## Maximum-range and migration evidence

PocketIC admitted `0-255`, verified all 256 effective account entries, matched `[0, 128, 255]` normally, and preserved all entries through upgrade. Admission consumed `73,331,954` cycles in the deterministic environment. Reported stable memory was `58,785,792` bytes before and after because the virtual-memory pages were already allocated; total memory likewise remained `61,171,038` bytes. The protocol bound is therefore comfortable and no range-width decision document or cap was required.

The exact pre-range fixture `tests/fixtures/event_horizon_pre_range_c3cb4b4_debug.wasm` was built at the starting commit and has SHA-256 `16a8342459c6092ceb9df63c38fab2759817a2fd9e3acacc8c82be1918ea8515`. Before upgrade, the regression established an account subscription, global subscription, initialized pricing, frozen next price, Ledger cursor progress, polling mode, and pending CMC transfer identity. Upgrade preserved all old state and account/global current and next values, derived the range price, created no phantom range entries, and subsequently admitted a new range.

## Validation results

- `npm ci` — passed.
- `cargo run -p xtask -- check` — passed: static checks, formatting, Clippy with warnings denied, workspace tests, 38 backend Rust unit tests, and 12 frontend tests.
- `DFX_IDENTITY=codex_local cargo run -p xtask -- pocketic` — passed: 24/24 integration tests.
- `DFX_IDENTITY=codex_local cargo run -p xtask -- local-smoke` — passed: 24/24 integration tests.
- `cargo run -p xtask -- security` — passed: zero vulnerabilities; four existing allowed unmaintained transitive warnings; deny advisories/bans/licenses/sources and OSV passed.
- `cargo run -p xtask -- release` — passed; production export audits and artifact checksum verification passed.
- `./tools/scripts/docker-build` — passed; canonical artifact checksum verification passed.
- `cargo run -p xtask -- repro` — passed: `release artifacts match across two clean builds`.
- `DFX_IDENTITY=codex_local icp build -e local` — passed: `Canisters built successfully`.
- `./tools/scripts/source-manifest generate && ./tools/scripts/source-manifest verify` — passed for every tracked source, including this report and the exact baseline fixture.

Canonical Docker Wasm SHA-256:

- backend: `ed62e91434282db7a4450ea12e1540baef486bdf3f4e2a895a4dd7de8d1c488f`
- frontend: `7efa050c60a1ec0a5d19604dc81fb6fae7da5e94d457f92cee5246bca758fe89`

Production application surfaces remain:

- backend: only `canister_query get_pricing` (9 total system/application Wasm exports);
- frontend: only `canister_query http_request` (7 total system/application Wasm exports).

## Security and remaining limitations

The maximum admission expansion is protocol-bounded to 256 stable-map merges. Range width creates neither Ledger work nor extra pokes: Ledger crawling remains global, matching remains one destination lookup, and each subscriber receives at most one best-effort poke per completed poll. Range admission uses one 20-ICP reference tier regardless of width and still requires an authorized Faucet payout plus complete Historian evidence for the exact route. No quotas, accounting, priorities, retries, queues, archive traversal, Index dependency, administrative API, or production debug method was added.

Existing documented limitations remain: notifications are non-authoritative and best-effort; subscribers must independently reconcile; Ledger archives are not traversed; pricing observes only successful daily CMC samples; and controller removal remains a separate irreversible post-observation operation.
