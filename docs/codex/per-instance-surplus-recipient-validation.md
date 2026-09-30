# Per-instance surplus recipient validation

Validation completed on 2026-09-30 from the merged starting revision
`c30401c8afa69178eadf01acabe0240ca800390c`. The implementation was first
committed as `e327bfd694d77140f84146dc3add6a0690372b44`; after the final production
backend hash was established, the frontend registry pin and source manifest
were finalized in canonical artifact source revision
`4fda8c12b8de4899a2b18edb057a3e540f95ee29`.

## Architecture

The production compile-time `SURPLUS_CANISTER` setting was removed. The
production constructor and stable instance configuration now carry the
optional immutable recipient:

```candid
type InitArgs = record {
  observed_ledger : principal;
  sns_root : opt principal;
  surplus_canister : opt principal;
};
```

`null` disables adaptive surplus diversion. `opt principal` enables the
existing adaptive policy and derives the recipient canister's default ICP
`AccountIdentifier` with a zero subaccount. The destination is independent of
the observed ledger and optional SNS Root. The ICP Ledger, CMC, NNS Governance,
Jupiter Faucet, and Jupiter Historian remain Wasm-level trust anchors;
`observed_ledger`, `sns_root`, and `surplus_canister` are immutable
instance-level configuration.

The production runtime reads the recipient from stable `InstanceConfig`.
`post_upgrade` remains argument-free, no production setter or administrative
method was added, and no schema migration framework was introduced. Existing
`PlannedSurplus` and pending-transfer records continue to freeze destination,
memo, amount, fee, and transfer identity before value moves.

The canonical ICP install arguments remain CMC-only:

```candid
(record {
  observed_ledger = principal "ryjl3-tyaaa-aaaaa-aaaba-cai";
  sns_root = null;
  surplus_canister = null;
})
```

## Behavioral validation

All commands below passed against canonical source revision `4fda8c12…`:

- `cargo test --locked -p xtask`: 18 passed.
- `cargo run -p xtask -- test_unit`: 4 suites, 88 tests passed. The parsed
  total comprised 76 Rust/xtask tests and 12 frontend tests.
- `cargo run -p xtask -- test_pocketic_integration`: 74 ignored tests
  discovered, 74 exact isolated processes passed, 0 failed.
- `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all`: 78 suites and 162
  tests passed: 88 repository/unit/frontend plus 74 PocketIC.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `./tools/scripts/security-scan`: passed. Cargo audit reported only the four
  existing reviewed maintenance warnings; cargo-deny checks passed, npm audit
  found zero vulnerabilities, and OSV found no unfiltered issue.

The surplus architecture is covered directly by:

- `disabled_destination_preserves_single_leg_funding_and_resets_policy`:
  `None` is exposed by `get_instance`, keeps funding CMC-only, and leaves the
  policy disabled/reset.
- `retained_top_up_completes_before_surplus_can_leave`: `Some(recipient)` is
  exposed by `get_instance`, starts conservatively at policy level zero, and
  transfers surplus to the recipient's derived default ICP account only after
  the retained leg.
- `same_production_wasm_supports_distinct_immutable_surplus_recipients`: the
  exact same production Wasm bytes/module hash were installed in three
  canisters with recipient A, recipient B, and `None`; the instances reported
  distinct immutable values and derived distinct recipient accounts. The
  configured values also survived argument-free current-Wasm upgrades.
- `production_constructor_rejects_invalid_surplus_recipients`: anonymous,
  management-canister, and self recipients all trapped at install.
- `sns_current_schema_profile_subscription_and_cursor_survive_upgrade` and
  `subscription_and_cursor_survive_upgrade`: current stable configuration,
  including the recipient, survived ordinary upgrades.
- `retained_plan_carries_original_surplus_identity_across_config_change`,
  `retained_transfer_plan_with_surplus_survives_upgrade`,
  `disabling_destination_does_not_cancel_pending_surplus_identity`, and
  `uncertain_surplus_transfer_stays_bound_to_original_destination`: planned or
  pending value remained bound to its frozen destination through later debug
  configuration changes and upgrades.

The production export test and canonical Wasm audit both passed. The backend
has ten total Wasm exports and exactly `canister_query get_instance` and
`canister_query get_pricing` as application methods, with no production update
method. The frontend has seven total exports and exactly
`canister_query http_request` as its application method.

## Frontend verification

The static instance registry now includes `surplusCanisterId` next to the
backend principal, observed ledger, SNS Root, and expected backend Wasm hash.
The canonical ICP entry pins `surplusCanisterId: null`; planned IO also remains
null. Frontend configuration verification compares
`get_instance().surplus_canister` with the reviewed registry value before
declaration construction, and the information panel displays the reviewed
recipient or `none`. The frontend exposes no mutation path.

## Canonical artifacts

`npm run verify:reproducible-artifacts` completed with two independent
no-cache Docker builds producing byte-identical files. The initial BuildKit
attempt's second image failed inside the pinned `cargo install ic-wasm` step
with exit 101 while the host filesystem had only 10 GiB free; it did not reach
Event Horizon compilation and did not produce an artifact mismatch. After
removing only repository build output, inactive BuildKit cache, and four
confirmed task-created untagged artifact images, the unchanged verification
script was rerun with `DOCKER_BUILDKIT=0` so intermediate compiler layers were
released between its two still-uncached builds. That run passed and produced
the same backend hash already observed in the successful clean BuildKit build.

The ordinary `./tools/scripts/docker-build` then passed from the clean canonical
source revision and populated `release-artifacts/`. The manifest verified all
three entries. `DFX_IDENTITY=codex_local icp build -e local` reported
`Canisters built successfully`; the canonical Docker build and manifest check
were rerun afterward and restored identical reviewed artifacts.

- Previous backend SHA-256:
  `91a12fe2f63edb5f7a3bab311a34294195a45bdde8df6eae3f83d3694387ad77`.
- New backend SHA-256:
  `019cee88c4933cbf929912a33a1120de0afb622f6311519d96662eda882ddd61`.
- New frontend SHA-256:
  `f37674d92071594302d4c30c822b77cbecf7d86facb55b1f22b1279e69bc5aee`.
- `build-info.json` SHA-256:
  `25e43f130e39381892835cf6144a04b0e80c5cf552e1350bb8f5e3431c478927`.
- Deterministic canonical release archive SHA-256:
  `5dd9e2f169264956d49087930d64ffe66bcea7fd0fa887dd5d715d93edf9461c`.

The archive hash is reproducible with:

```bash
tar --sort=name --mtime='@1700000000' --owner=0 --group=0 \
  --numeric-owner --format=ustar -C release-artifacts -cf - \
  build-info.json event_horizon.wasm event_horizon_frontend.wasm \
  release-artifacts.sha256 | gzip -n | sha256sum
```

## Safety

No deploy, reinstall, upgrade, controller change, transfer, cycle top-up,
Jupiter alias publication, or other mainnet mutation occurred.
