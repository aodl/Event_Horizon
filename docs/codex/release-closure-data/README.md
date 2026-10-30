# Release-closure reproduction

Validated recipe/source: d13d1138e2fde4af18bd0d3f51f777470037f93f.
See ../cycle-optimisation-release-closure.md for interpretation and blockers.

The CSVs retain generated line endings and distinguish package-only z,
standalone frontend z, full-workspace z and original2. `screen-receipt.json`
binds source, artifact identities and log digests. `status.json` binds the complete
supported-gate command ledger. `closure-receipt.json`, canonical-build-info.json
and canonical-artifacts.sha256 record populated release metadata. Advisory JSON
and final retrieval timestamps are retained without locally modifying ranges.

To repeat screening at9528da7 with unchanged generated assets, set pinned Rust,
SOURCE_DATE_EPOCH and canonical remapping flags as in build-release. Build both
packages at 2; for package-only z add Cargo config
`profile.release.package.event-horizon-frontend.opt-level="z"`. For full-workspace
z build both packages with CARGO_PROFILE_RELEASE_OPT_LEVEL=z in a distinct target
directory; for the standalone control select only event-horizon-frontend. Shrink
with ic-wasm 0.9.7, audit exports and preserve separate files. Use the existing
`frontend_profile_work` fixture from cycle-optimisation-experiments.md, selecting
one profile at a time. Package-only z is labelled z by that fixture; its separate
CSV filename identifies its actual recipe. `sizes.json` contains local raw and
post-shrink sizes for initial controls; final selected size is in the receipt.

All-public-asset probe (after test_all has compiled the pinned native dependencies):

```bash
DFX_IDENTITY=codex_local POCKET_IC_BIN=/path/to/official/pocket-ic-15 \
  POCKET_IC_MUTE_SERVER=1 MALLOC_ARENA_MAX=2 \
  python3 docs/codex/release-closure-data/asset-probe/run.py \
  /path/to/9528da7-profile2/event_horizon_frontend.wasm \
  release-artifacts/event_horizon_frontend.wasm
```

The first module must hash46d7b10673b7c37dce7ec773a0829bf104723b57b7fe2370d5838bc8df3b4a24
and can be reconstructed with the supported canonical build at9528da7. The probe
uses ordinary limits, fixed initial time, complete asset-body comparison and
certificate-header assertions. Stable/linear memory are reported separately.
It does not add a production query or claim cryptographic proof verification or
query-cycle savings. Raw result: full-assets.csv.

The standalone panic probe has its own native-only lockfile, outside the production
workspace. Run `cargo run --locked --manifest-path docs/codex/release-closure-data/panic-probe/Cargo.toml`.
Its191 native cases complement source inspection; they are not Miri or a general
safety proof. No advisory filter is activated by either probe.
