#!/usr/bin/env bash
# build_census.sh NAME -- build quirin-audit at the worktree's current tree into
# a private target directory and copy it to private/cg22/quirin-audit-NAME, so
# target/release (the BEFORE binary, still running) is not replaced.
# `census` was built with legacy_indirect_calls.rs ADMIT = false (parity rows,
# no rewrite); `after` with ADMIT = true.
set -u
name="${1:-census}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
CARGO_TARGET_DIR="$repo/private/cg22-target" CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 \
  cargo build --release -p qcore-audit --bin quirin-audit > "$repo/private/cg22/build-$name.log" 2>&1
code=$?
echo $code > "$repo/private/cg22/build-$name.exit"
if [ "$code" = 0 ]; then
  cp "$repo/private/cg22-target/release/quirin-audit" "$repo/private/cg22/quirin-audit-$name"
  git -C "$repo" rev-parse HEAD > "$repo/private/cg22/quirin-audit-$name.commit"
  git -C "$repo" status --short > "$repo/private/cg22/quirin-audit-$name.dirty"
fi
