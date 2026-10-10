#!/usr/bin/env bash
# targeted.sh NAME TEST... -- W45-CALLEE-CASCADE-RESIDUAL round 3 (from CG22-CALLBACK-REGISTRY-CONTRACT): run the named
# integration tests (`PACKAGE:TEST`, e.g. `qcore-compiler:qcore_contract_corpus`)
# under the shared heavy lock, each one's output and exit to
# private/w45r3/targeted-NAME-PACKAGE-TEST.{log,exit}; nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/w45r3"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
for spec in "$@"; do
  package="${spec%%:*}"; test="${spec#*:}"
  log="$out/targeted-$name-$package-$test"
  if [ "$test" = lib ]; then
    flock "$lock" cargo test -p "$package" --lib > "$log.log" 2>&1
  else
    flock "$lock" cargo test -p "$package" --test "$test" > "$log.log" 2>&1
  fi
  echo $? > "$log.exit"
  echo "$spec $(cat "$log.exit") $(grep -E '^test result' "$log.log" | tail -1)"
done
