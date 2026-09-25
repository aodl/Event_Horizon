# Polling documentation validation

## Revisions and scope

- Starting commit: `ed32cbebbaa8889cb9c76df4dd07a8c9aadff6db`
- Validated documentation/static-check commit: `4d360d9ab847a8c59d9121cde875eb7bf616edba`
- Final evidence commit: the commit containing this report and regenerated `MANIFEST.sha256`; its SHA is recorded in the final handoff because a Git commit cannot contain its own object ID.
- Branch: `codex/polling-docs`

Changed files before evidence generation:

```text
README.md
SPEC.md
docs/architecture.md
docs/deployment.md
docs/operational-backend.md
tools/static-check.py
```

This report and `MANIFEST.sha256` complete the evidence commit. No backend Rust, frontend JavaScript, Candid, cadence constants, scheduler logic, pricing, funding, stable-memory, or Docker build logic changed.

## Documentation results

- The root README now has a prominent Adaptive polling cadence section with the complete Reserve Protection, Economy, Standard, Fast, Very Fast, and Continuous table.
- The README explains liquid cycles, stateful hysteresis, immediate upward acceleration, lower exit thresholds, the Standard example, added-delay semantics, non-overlapping Continuous polling, Reserve Protection recovery, and the distinction from surplus thresholds.
- `docs/operational-backend.md` now contains the primary detailed polling-cadence explanation, including one-shot timers, single-flight polling, Reserve Protection maintenance, and immediate reevaluation after a successful CMC mint.
- `docs/deployment.md` now presents initial funding as an operator choice for the latency tier under acceptance testing rather than requiring immediate Continuous operation. It includes tier examples and separates the 100 T Continuous threshold from the currently disabled 150 T surplus-health policy.
- Deployment guidance now shows public status for `eo6ei-gaaaa-aaaar-qchra-cai`, explains that cadence is inferred from balance, previous mode, and hysteresis, and does not claim that `canister_status` reports the internal mode.
- Deployment guidance now includes readable project-name and raw-principal `get_pricing` calls with the checked-in Candid interface, concise field meanings, and the legitimate first-deployment stale carry-forward case.
- The stale frontend-discovery statement in `SPEC.md` now describes the permanent backend principal embedded in the JavaScript asset and frontend Wasm. The tracked current/live documentation audit found no remaining stale discovery markers.
- `tools/static-check.py` now requires the canonical operational guide to retain all mode names and key entry/exit threshold markers without duplicating cadence logic in Python. Rust constants remain executable truth and `SPEC.md` remains normative.

## Validation results

`cargo run -p xtask -- check` passed:

- static/source invariants passed;
- Rust formatting passed;
- warnings-denied workspace Clippy passed;
- 46 backend Rust unit tests passed;
- ordinary workspace tests passed, with the 37 intentionally ignored PocketIC scenarios not rerun for this documentation-only change;
- 13 frontend JavaScript tests passed.

`cargo run -p xtask -- canonical` passed and verified the canonical artifact manifest. The uncompressed production Wasm hashes remain:

```text
d5a66897cb914488df53799842a89912f65a0688793b3eb198b064ed7d5c202b  event_horizon.wasm
3f19289a19a940ed023d05957a24c3e2d0b50402ddfffd978f08daa7aa01df63  event_horizon_frontend.wasm
```

Both hashes are unchanged from the starting commit. Documentation/static-check metadata changed as expected; no production Wasm input affecting executable behavior changed.

`tools/scripts/source-manifest generate` was run after documentation and evidence were complete, and `tools/scripts/source-manifest verify` passed. The branch was clean after the evidence commit.

No mainnet canister was deployed or upgraded.
