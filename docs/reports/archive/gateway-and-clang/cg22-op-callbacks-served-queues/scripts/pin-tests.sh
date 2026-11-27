#!/usr/bin/env bash
# pin-tests.sh NAME -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the contract pin
# tables (`compiler/tests/{contract_loader,qcore_contract_corpus}.rs`) and the
# audit's shipped-policy digest test under the shared heavy lock, logs and
# exits into private/cg22op/pins-NAME-*. Nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22op"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
for test in qcore_contract_corpus contract_loader; do
  flock "$lock" cargo test -p qcore-compiler --test "$test" > "$out/pins-$name-$test.log" 2>&1
  echo $? > "$out/pins-$name-$test.exit"
  echo "$test $(cat "$out/pins-$name-$test.exit")"
done
flock "$lock" cargo test -p qcore-audit --test source_contract_config > "$out/pins-$name-source_contract_config.log" 2>&1
echo $? > "$out/pins-$name-source_contract_config.exit"
echo "source_contract_config $(cat "$out/pins-$name-source_contract_config.exit")"
