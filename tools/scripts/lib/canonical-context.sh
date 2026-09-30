#!/usr/bin/env bash

# Private helper sourced by the canonical Docker wrappers.
canonical_context_prepare() {
  local root="$1" untracked
  CANONICAL_SOURCE_SHA="$(git -C "$root" rev-parse --verify 'HEAD^{commit}')"

  if ! git -C "$root" diff --quiet --ignore-submodules -- \
      || ! git -C "$root" diff --cached --quiet --ignore-submodules --; then
    echo "canonical builds require a clean tracked source tree" >&2
    return 1
  fi
  untracked="$(git -C "$root" ls-files --others --exclude-standard)"
  if [[ -n "$untracked" ]]; then
    echo "canonical builds reject untracked, non-ignored source files:" >&2
    printf '%s\n' "$untracked" >&2
    return 1
  fi

  (cd "$root" && ./tools/scripts/source-manifest verify)
  CANONICAL_CONTEXT="$(mktemp -d "${TMPDIR:-/tmp}/event-horizon-canonical.XXXXXX")"
  if ! git -C "$root" archive --format=tar "$CANONICAL_SOURCE_SHA" \
      | tar -xf - -C "$CANONICAL_CONTEXT"; then
    rm -rf "$CANONICAL_CONTEXT"
    CANONICAL_CONTEXT=""
    return 1
  fi
  if ! (cd "$CANONICAL_CONTEXT" && ./tools/scripts/source-manifest verify); then
    rm -rf "$CANONICAL_CONTEXT"
    CANONICAL_CONTEXT=""
    return 1
  fi
  export CANONICAL_SOURCE_SHA CANONICAL_CONTEXT
  echo "Canonical source revision: $CANONICAL_SOURCE_SHA"
  echo "Canonical context: git archive of $CANONICAL_SOURCE_SHA"
}

canonical_context_cleanup() {
  if [[ -n "${CANONICAL_CONTEXT:-}" && -d "$CANONICAL_CONTEXT" ]]; then
    rm -rf "$CANONICAL_CONTEXT"
  fi
  CANONICAL_CONTEXT=""
}
