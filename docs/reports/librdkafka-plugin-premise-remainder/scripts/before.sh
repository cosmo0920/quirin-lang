#!/usr/bin/env bash
# before.sh [NAME] -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: build the BEFORE
# binary from an exported copy of the base revision (private/lppr/base-src,
# written by `git archive 711287771`, its third_party/libevent a link to this
# worktree's submodule checkout at the pinned d82464a2), so the corpus
# binary's compiled-in repository root (the CARGO_MANIFEST_DIR its pinned route
# reads its contracts from) holds the base contracts however this worktree
# changes while the run waits for the shared lock. Keep it as
# private/lppr/bin/before (built once) and measure it on every route with the
# null property, with REPO set to the base tree, so the system and slice
# routes read the base's profile, scripts and provider contracts too; the runs
# go to private/lppr/NAME* (default before2). Exits to files; nothing piped.
#
# The first attempt (`before`, REPO left at this worktree, no libevent link)
# is retained and discarded: the corpus route stopped at the libevent source
# pin (the empty submodule directory resolved to this worktree's HEAD) and
# the system route read this worktree's changed contracts.
set -u
name="${1:-before2}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/lppr"
base="$out/base-src"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
[ -f "$base/Cargo.toml" ] || exit 2
if [ ! -f "$out/bin/before" ]; then
  cd "$base" || exit 2
  export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
  export QCORE_AUDIT_BUILD_COMMIT=7112877710e6c7d5887ba16ed6126773cf19dd7a
  flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
    --target-dir "$out/target-before" > "$out/build-before.log" 2>&1
  echo $? > "$out/build-before.exit"
  [ "$(cat "$out/build-before.exit")" = 0 ] || exit 1
  cp "$out/target-before/release/quirin-audit" "$out/bin/before"
  sha256sum "$out/bin/before" > "$out/bin/before.sha256"
fi
REPO="$base" OUT="$out" NULL=1 "$here/measure-binary.sh" "$name" "$out/bin/before" > "$out/$name-measure.out" 2>&1
echo $? > "$out/$name-measure.step.exit"
