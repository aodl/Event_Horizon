# Documentation index

[`SPEC.md`](../SPEC.md) is the normative protocol. Current explanatory documentation is grouped by concern; `docs/codex/` and `docs/provenance/` are historical evidence rather than live protocol definitions.

## Mainnet deployment identities

| CLI name | Existing canister ID |
| --- | --- |
| `event_horizon` (backend) | `eo6ei-gaaaa-aaaar-qchra-cai` |
| `event_horizon_frontend` (frontend) | `ej7c4-lyaaa-aaaar-qchrq-cai` |

The [deployment runbook](operations/deployment.md#canister-name-resolution)
explains how the `ic` environment resolves these names through the local mapping
file, including setup on a fresh checkout. The
[backend README](../canisters/event-horizon/README.md#mainnet-identity) and
[frontend README](../canisters/frontend/README.md#mainnet-identity) document each
component's identity and role.

## Architecture

- [Overview](architecture/overview.md)
- [Trust model](architecture/trust-model.md)
- [Stable memory](architecture/stable-memory.md)
- [Production surface](architecture/production-surface.md)
- [Funding, economics, and pricing](architecture/economics-pricing.md) — the
  authoritative explanation of productive endowments, dynamic prices, pooled
  operating funding, and the planned protocol kickstart

## Development and testing

- [Testing](development/testing.md)
- [Cadence calibration](development/cadence-calibration.md)
- [xtask commands](../tools/xtask/README.md)
- [Backend canister README](../canisters/event-horizon/README.md)
- [Frontend canister README](../canisters/frontend/README.md)

## Security

- [Dependency scanning](security/dependency-scanning.md)
- [Trust model](architecture/trust-model.md)

## Operations and deployment

- [Deployment — authoritative current first-release procedure](operations/deployment.md)
- [Deploying another immutable instance](operations/deploying-an-instance.md)
- [Acceptance observation](operations/acceptance-observation.md)
- [Controller removal](operations/controller-removal.md)
- [Operational backend](operations/operational-backend.md)
- [Reproducible builds](operations/reproducible-builds.md)

## Integration and history

- [Subscriber guide](subscriber-guide.md)
- [Index wake-up interoperability validation](codex/index-poke-validation.md)
- [Checkpoint history](../CHECKPOINT.md)
- [Landing ledger-counter validation](codex/ledger-counter-validation.md)
- [Landing hero and sampled-rate validation](codex/ledger-hero-refinement-validation.md)
- [PocketIC process-isolation validation](codex/pocketic-process-isolation-validation.md)
- [Jupiter workflow alignment validation](codex/jupiter-workflow-alignment-validation.md)
- [`docs/codex/`](codex/)
- [`docs/provenance/`](provenance/)

The [resumed pre-release report](codex/pre-release-resume.md) distinguishes timestamped live observations from historical validation and lists unresolved release blockers.
