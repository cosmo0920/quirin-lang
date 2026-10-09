#!/usr/bin/env bash
# build.sh NAME [FEATURES] -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (from DEEP-AST-TRANSPORTS-AND-WALL-TIME) (adapted from
# RDHTTP-HEADER-SUMMARY-GAP's scripts/build.sh): under the shared heavy lock,
# build the release quirin-audit of the checked-out tree (with cargo FEATURES
# when given) into a private target directory (private/cg10r2/target-rel, or
# target-FEATURES), so target/release/quirin-audit -- the path
# run_pinned_audit.py runs, which measure-binary.sh fills with a saved binary --
# is never replaced by a build; then keep a copy as private/cg10r2/bin/NAME with
# its digest, and the tree's HEAD and tracked-file status beside it.
# Exits to files; nothing piped.
set -u
name="$1"
features="${2:-}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg10r2"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/build-$name.head"
git status --porcelain --untracked-files=no > "$out/build-$name.dirty"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
if [ -z "$features" ]; then
  target="$out/target-rel"
  flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
    --target-dir "$target" > "$out/build-$name.log" 2>&1
else
  target="$out/target-$(echo "$features" | tr '/,' '__')"
  flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit --features "$features" \
    --target-dir "$target" > "$out/build-$name.log" 2>&1
fi
echo $? > "$out/build-$name.exit"
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
cp "$target/release/quirin-audit" "$out/bin/$name"
sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
cat "$out/bin/$name.sha256"
