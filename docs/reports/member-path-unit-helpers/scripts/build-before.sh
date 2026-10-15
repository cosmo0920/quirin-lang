#!/usr/bin/env bash
# build-before.sh -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: build the BEFORE
# release quirin-audit from an export of the base (`git archive cd1239503`,
# the frozen prediction over main d708641bc: product code identical to main)
# in private/mpuh/before-src, so the card's edits in the worktree never reach
# the BEFORE binary while the build waits on the shared heavy lock. The
# export's compiled-in CARGO_MANIFEST_DIR reads the export's (main's)
# contracts. Keeps a copy as private/mpuh/bin/before. Exits to files.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpuh"
src="$out/before-src"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
cd "$src" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
echo cd1239503 > "$out/build-before.head"
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
  --target-dir "$out/target-before" > "$out/build-before.log" 2>&1
echo $? > "$out/build-before.exit"
[ "$(cat "$out/build-before.exit")" = 0 ] || exit 1
cp "$out/target-before/release/quirin-audit" "$out/bin/before"
sha256sum "$out/bin/before" > "$out/bin/before.sha256"
cat "$out/bin/before.sha256"
