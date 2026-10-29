#!/usr/bin/env bash
# Rebuild historical compatibility inputs in a fresh source directory.
# Usage: rebuild-54d8b4b.sh NEW_DIRECTORY IC_WASM_0_9_7_BINARY
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
build_dir="$1"
ic_wasm="$(realpath "$2")"
[[ "$("$ic_wasm" --version)" == 'ic-wasm 0.9.7' ]]
mkdir "$build_dir"
build_dir="$(realpath "$build_dir")"
git archive 54d8b4ba99e6392cfbfa3a5d064ce8f57f8378d7 | tar -x -C "$build_dir"
cd "$build_dir"
export SOURCE_DATE_EPOCH=1700000000 DFX_IDENTITY=codex_local
export RUSTFLAGS="--remap-path-prefix=$build_dir=/build --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
export CARGO_TARGET_DIR="$build_dir/target"
for kind in production debug; do
  extra=()
  if [[ "$kind" == debug ]]; then extra=(--features debug_api); fi
  cargo +1.94.1 build --locked --release --target wasm32-unknown-unknown \
    -p event-horizon -p event-horizon-frontend "${extra[@]}"
  "$ic_wasm" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/event_horizon.wasm" \
    -o "$build_dir/$kind.wasm" shrink
done
printf '%s  production.wasm\n%s  debug.wasm\n' \
  e012e7138cfe2d251bd24edf868cb74aa5fd35856412facd926b65ad519cdcca \
  43fb4206372a42c1ef30238101fdc41d7ef44ccc22945461c6eed28e56bfef7c | sha256sum -c -
printf 'Compare these exact bytes with %s/tests/fixtures/event_horizon_54d8b4b_{production,debug}.wasm\n' "$root"
