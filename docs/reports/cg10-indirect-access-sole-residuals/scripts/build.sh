#!/usr/bin/env bash
# build.sh NAME [census] -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: under the
# shared heavy lock, build the release quirin-audit into target/release (the
# path scripts/run_pinned_audit.py runs) and keep a copy at
# private/cg10res/NAME.quirin-audit; with `census`, also build the
# measurement-only census binary (`--features qcore-c-gateway/indirect-census`)
# into private/cg10res/target-census. Every exit is written to a file under
# private/cg10res; nothing is piped.
set -u
name="$1"
census="${2:-}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg10res"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
cp target/release/quirin-audit "$out/$name.quirin-audit"
sha256sum "$out/$name.quirin-audit" > "$out/$name.quirin-audit.sha256"
if [ "$census" = census ]; then
  CARGO_TARGET_DIR="$out/target-census" flock "$lock" cargo build --release -p qcore-audit \
    --bin quirin-audit --features qcore-c-gateway/indirect-census > "$out/build-$name-census.log" 2>&1
  echo $? > "$out/build-$name-census.exit"
fi
echo "build $(cat "$out/build-$name.exit") census $(cat "$out/build-$name-census.exit" 2>/dev/null)"
