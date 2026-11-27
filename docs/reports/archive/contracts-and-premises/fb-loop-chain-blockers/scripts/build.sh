#!/usr/bin/env bash
# build.sh NAME [FEATURES] -- FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS (copied from FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME, itself from FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, FB-QUANT-CONTRACT-CENSUS and
# NULL-TRIAGE-UNRESOLVED-PATHS's scripts/build.sh; the FEATURES argument from
# UNION-OPAQUE-RECORD-WRITES-BY-TYPE's): under the shared heavy lock, build the
# release quirin-audit of the checked-out tree (with cargo FEATURES when given,
# e.g. qcore-c-gateway/indirect-census) into a private target directory
# (private/fblb/target-rel, or target-FEATURES), then keep a copy as
# private/fblb/bin/NAME with its sha256. Exits to files; nothing piped.
set -u
name="$1"
features="${2:-}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fblb"
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
