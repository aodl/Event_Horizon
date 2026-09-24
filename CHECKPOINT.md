# Event Horizon repository status

`SPEC.md` is the normative protocol. Repository evidence is divided into three stages so historical claims are not confused with checks executed later.

## Checkpoint 03 historical import

Checkpoint 03 was the supplied release-hardening source package. Its original handover reported only static checks available in the producing environment and explicitly did not claim Rust, PocketIC, Docker, security, or `icp` execution. The untouched import is commit `53b9909` and tag `checkpoint-03-import`. Its original 77-file checksum list is preserved at `docs/provenance/checkpoint-03-MANIFEST.sha256`.

## Codex initial validation

[`docs/codex/initial-validation.md`](docs/codex/initial-validation.md) is the preserved report for the first executable validation pass. It records the imported ZIP hash, environment, baseline failures, source fixes, exact commands, 21 Rust unit tests, 12 PocketIC tests, security results, canonical Docker Wasm hashes, reproducibility evidence, and the then-unresolved CMC transfer-expiry decision.

## Post-validation hardening

[`docs/codex/post-validation-hardening.md`](docs/codex/post-validation-hardening.md) is the current handover. This pass narrowly changes the trusted ICP Ledger value-moving transfer to guaranteed-response semantics, resolves expired transfer identities while documenting the residual pathological risk, immediately reconsiders polling cadence after successful CMC minting, adds the pinned-boundary interleaving regression, and refreshes source provenance.

The root `MANIFEST.sha256` describes the current tracked source. Generate or verify it with `tools/scripts/source-manifest`. Canonical source handoff identity is the final Git commit SHA together with the exact SHA-256 of `git archive --format=zip`. The ZIP attached to the ChatGPT review was repackaged and therefore was not byte-identical to the previously reported Git archive.

That post-validation hardening stage made no mainnet deployment, Jupiter Faucet `X` alias publication, controller removal, or production interface expansion.

## Global subscriptions and dynamic pricing

Development from the validated-core tag `event-horizon-core-validated` is recorded in [`docs/codex/global-pricing-validation.md`](docs/codex/global-pricing-validation.md). This stage adds a direct global subscriber registry, `poke([])` semantics with account-hint precedence, CMC-based rolling-floor pricing, monthly frozen epochs, additive stable memory IDs 4–6, and the sole production application query `get_pricing`. The prior validation reports and Checkpoint 03 manifest remain historical evidence.

## Range subscriptions

Development from `c3cb4b41f390cbe7ce81e8ef6976ff2527ee5161` is recorded in [`docs/codex/range-subscriptions-validation.md`](docs/codex/range-subscriptions-validation.md). This stage adds the bounded inclusive range grammar, admission-time expansion into the existing account map, independent 20-ICP pricing, frontend memo support, exact pre-range upgrade coverage, and validation of the maximum 256-account range. It ends at validated commit `304b26e9e6facc6f8812e61c56f98e712e05f803`.

## Adaptive surplus diversion

Development from exact range baseline `304b26e9e6facc6f8812e61c56f98e712e05f803` is recorded in [`docs/codex/surplus-diversion-validation.md`](docs/codex/surplus-diversion-validation.md). This stage adds the optional immutable receiver (currently disabled in production), seven-day observed-liquid-cycles controller, retained-first FundingStateV2, additive stable IDs 7–8, and deterministic surplus Ledger recovery. The exact baseline fixture exercises old Idle, TransferPending, and NotifyPending migration; current-state upgrades cover every new pending phase. Production surfaces remain backend `get_pricing` and frontend `http_request` only.

The root `MANIFEST.sha256` is regenerated only after the surplus implementation and final evidence are complete. Historical validation documents and the Checkpoint 03 manifest remain unchanged evidence. No stage in this chain deployed to mainnet, published the Jupiter Faucet `X` alias, or removed/changed controllers.

## Surplus identity hardening

The narrowly scoped follow-up from `4d21a35822d0b9ce04fd097a5e99abb39ccb23d3` is recorded in [`docs/codex/surplus-identity-hardening.md`](docs/codex/surplus-identity-hardening.md). It freezes the destination account identifier and `SURPLUS1` memo in every split plan before the retained transfer begins, and makes the eventual pending surplus transfer fully self-contained across upgrades. Exact `304b26e…` migration remains intact; a newly tracked exact `4d21a35…` fixture proves an uncertain accepted transfer stays bound to destination A after upgrading and changing debug configuration to B. Policy, economics, stable-memory IDs, subscription/pricing/polling behavior, and production surfaces are unchanged.
