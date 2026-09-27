#!/usr/bin/env bash
# W46-ACCESS-PATH-CENSUS: the final corpus arbiter on the default (census-off)
# release build, and its identity against the baseline.
# usage: arbiter.sh OUT_DIR COMMIT   (OUT_DIR must not exist; logs beside it)
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$1"
commit="$2"
logs="$out.logs"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit > "$logs/build.log" 2>&1
echo $? > "$logs/build.exit"
python3 docs/reports/w43-return-expression/scripts/run_audit.py "$out" "$commit" --jobs 12 > "$logs/run.log" 2>&1
echo $? > "$logs/run.exit"
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py "$out" \
  docs/reports/w46-external-callee-contracts/integration/artifacts \
  --root-alias "$repo=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" \
  > "$logs/identity.json" 2>&1
echo $? > "$logs/identity.exit"
