#!/usr/bin/env bash
# targeted.sh NAME -- PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: the
# landing-race targeted check after a records-only rebase onto a main whose new
# commits are disjoint from this card's code (integrator's list). Run as
#   flock <heavy.lock> bash targeted.sh NAME
# Every step's output and exit go to private/ppb/targeted-NAME/; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
report="$repo/docs/reports/plugin-package-build-write"
out="$repo/private/ppb/targeted-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
git rev-parse HEAD > "$out/head"
echo "started $(date -u +%FT%T.%NZ)" > "$out/times"
step 00-check cargo check --workspace --all-targets
step 01-provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step 02-host-tests cargo test -p qcore-verifier-host --locked
step 03-matrix python3 docs/reports/remaining-d02/build_matrix.py . --gate "$report/gate.json"
step 04-python python3 -m unittest discover -s scripts/tests
step 05-index python3 scripts/build_docs_index.py
step 06-index-check python3 scripts/build_docs_index.py --check
echo "finished $(date -u +%FT%T.%NZ)" >> "$out/times"
grep -H . "$out"/*.exit
