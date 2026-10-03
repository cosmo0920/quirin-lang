#!/usr/bin/env bash
# build.sh NAME [FEATURES] -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION: under the
# shared heavy lock, build the release quirin-audit of the checked-out tree
# (with cargo FEATURES when given, e.g. qcore-c-gateway/indirect-census) into a
# private target directory (private/gapo/target-rel, or target-FEATURES), so
# target/release/quirin-audit -- the path run_pinned_audit.py runs, which
# measure-binary.sh fills with a saved binary -- is never replaced by a build;
# then keep a copy as private/gapo/bin/NAME. The target directory changes no
# compiled-in path: CARGO_MANIFEST_DIR is the checkout either way. (The BEFORE
# binary, bin/before, was built into target/release by the first version of
# this script.) Exits to files; nothing piped.
set -u
name="$1"
features="${2:-}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/gapo"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/build-$name.head"
git status --porcelain --untracked-files=no > "$out/build-$name.dirty"
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
