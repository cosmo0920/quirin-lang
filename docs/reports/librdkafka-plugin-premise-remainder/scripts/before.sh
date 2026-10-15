#!/usr/bin/env bash
# before.sh -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: build the BEFORE binary
# from an exported copy of the base revision (private/lppr/base-src, written
# by `git archive 711287771`), so its compiled-in repository root (the
# CARGO_MANIFEST_DIR the audit reads its contracts from at run time) holds the
# base contracts however this worktree changes while the run waits for the
# shared lock; keep it as private/lppr/bin/before and measure it on every route
# with the null property (measure-binary.sh). Exits to files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/lppr"
base="$out/base-src"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
[ -f "$base/Cargo.toml" ] || exit 2
cd "$base" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
export QCORE_AUDIT_BUILD_COMMIT=7112877710e6c7d5887ba16ed6126773cf19dd7a
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
  --target-dir "$out/target-before" > "$out/build-before.log" 2>&1
echo $? > "$out/build-before.exit"
[ "$(cat "$out/build-before.exit")" = 0 ] || exit 1
cp "$out/target-before/release/quirin-audit" "$out/bin/before"
sha256sum "$out/bin/before" > "$out/bin/before.sha256"
NULL=1 "$here/measure-binary.sh" before "$out/bin/before" > "$out/before-measure.out" 2>&1
echo $? > "$out/before-measure.step.exit"
