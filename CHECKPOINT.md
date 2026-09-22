# Event Horizon repository status

`SPEC.md` is the normative protocol. Repository evidence is divided into three stages so historical claims are not confused with checks executed later.

## Checkpoint 03 historical import

Checkpoint 03 was the supplied release-hardening source package. Its original handover reported only static checks available in the producing environment and explicitly did not claim Rust, PocketIC, Docker, security, or `icp` execution. The untouched import is commit `53b9909` and tag `checkpoint-03-import`. Its original 77-file checksum list is preserved at `docs/provenance/checkpoint-03-MANIFEST.sha256`.

## Codex initial validation

[`docs/codex/initial-validation.md`](docs/codex/initial-validation.md) is the preserved report for the first executable validation pass. It records the imported ZIP hash, environment, baseline failures, source fixes, exact commands, 21 Rust unit tests, 12 PocketIC tests, security results, canonical Docker Wasm hashes, reproducibility evidence, and the then-unresolved CMC transfer-expiry decision.

## Current post-validation state

[`docs/codex/post-validation-hardening.md`](docs/codex/post-validation-hardening.md) is the current handover. This pass narrowly changes the trusted ICP Ledger value-moving transfer to guaranteed-response semantics, resolves expired transfer identities while documenting the residual pathological risk, immediately reconsiders polling cadence after successful CMC minting, adds the pinned-boundary interleaving regression, and refreshes source provenance.

The root `MANIFEST.sha256` describes the current tracked source. Generate or verify it with `tools/scripts/source-manifest`. Canonical source handoff identity is the final Git commit SHA together with the exact SHA-256 of `git archive --format=zip`. The ZIP attached to the ChatGPT review was repackaged and therefore was not byte-identical to the previously reported Git archive.

No mainnet deployment, Jupiter Faucet `X` alias publication, controller removal, or production interface expansion is part of this status.
