#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
worker_bun="$(realpath "$(command -v "${BUN:-bun}")")"
[[ "$("$worker_bun" --version)" == 1.4.5 ]]
mkdir -p .local
worker_temp="$(mktemp -d "$PWD/.local/eh-artifact-test.XXXXXX")"
trap 'rm -rf -- "$worker_temp"' EXIT
# Exercise the same deterministic suites on Bun, including Node-style mocks.
env -i PATH="$PATH" HOME="$worker_temp" TMPDIR="$worker_temp" "$worker_bun" test --no-env-file \
  test/{accounts,config,core,file-store,icp,idl,vps-loop}.test.mjs
env -i PATH="$PATH" HOME="$worker_temp" TMPDIR="$worker_temp" "$worker_bun" build --no-env-file --compile \
  --target=bun-linux-x64 --no-compile-autoload-dotenv --no-compile-autoload-bunfig \
  test/runtime-harness.mjs --outfile "$worker_temp/runtime-harness"
BUN="$worker_bun" EH_TEST_COMPILED_HARNESS="$worker_temp/runtime-harness" node --test test/*.artifact.mjs
