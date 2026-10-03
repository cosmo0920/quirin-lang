#!/usr/bin/env bash
# build.sh NAME [FEATURES] -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION: under the
# shared heavy lock, build the release quirin-audit (with cargo FEATURES when
# given, e.g. qcore-c-gateway/route-profile, into a separate target directory
# private/gapo/target-FEATURES so the measured binary is never replaced), then
# keep a copy as private/gapo/bin/NAME. Exits to files; nothing piped.
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
git status --porcelain > "$out/build-$name.dirty"
if [ -z "$features" ]; then
  flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit > "$out/build-$name.log" 2>&1
  echo $? > "$out/build-$name.exit"
  built="$repo/target/release/quirin-audit"
else
  target="$out/target-$(echo "$features" | tr '/,' '__')"
  flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit --features "$features" \
    --target-dir "$target" > "$out/build-$name.log" 2>&1
  echo $? > "$out/build-$name.exit"
  built="$target/release/quirin-audit"
fi
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
cp "$built" "$out/bin/$name"
sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
cat "$out/bin/$name.sha256"
