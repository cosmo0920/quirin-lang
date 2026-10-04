#!/usr/bin/env bash
# targeted.sh NAME [CARGO TEST ARGS...] -- W45-CALLEE-CASCADE-RESIDUAL: the
# contract pin tests (default: qcore-compiler's contract_loader and
# qcore_contract_corpus) under the shared heavy lock; output and exit to
# private/w45ccr/targeted-NAME.{log,exit}. Nothing piped.
set -u
name="$1"; shift
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/w45ccr"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
if [ "$#" -eq 0 ]; then
  set -- -p qcore-compiler --test contract_loader --test qcore_contract_corpus
fi
flock "$lock" cargo test "$@" > "$out/targeted-$name.log" 2>&1
echo $? > "$out/targeted-$name.exit"
echo "targeted $name exit $(cat "$out/targeted-$name.exit")"
