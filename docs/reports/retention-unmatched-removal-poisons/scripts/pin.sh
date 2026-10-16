#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR -- RETENTION-UNMATCHED-REMOVAL-POISONS (from
# FB-EVENT-LOOP-RETENTION-AND-REGISTRATION's pin.sh): sync the provider
# inventories (under the shared heavy lock: it runs a cargo example), pin the
# two loop declarations at the three audited revisions (66910c10 the corpus
# clone, 046accf6 the system clone, the 90978ac6 slice export), then check
# the libevent crate's lock at each and verify every contract. Each step's
# exit is captured to its own file.
set -u
binary="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
logs="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
roots=(
  /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
  /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
)
tags=(66910c10 046accf6 90978ac6)
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
step sync-before flock "$lock" python3 providers/contracts.py sync
for i in 0 1 2; do
  step "pin-libevent-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate qcore-provider-libevent \
    --function mk_event_loop_create --function mk_event_loop_destroy
done
step sync-after flock "$lock" python3 providers/contracts.py sync
for i in 0 1 2; do
  step "check-libevent-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate qcore-provider-libevent --check
done
step verify-contracts "$binary" pin --verify-contracts
