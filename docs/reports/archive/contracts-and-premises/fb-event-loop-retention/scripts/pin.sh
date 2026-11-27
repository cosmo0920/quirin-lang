#!/usr/bin/env bash
# FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: pin the reviewed declarations at
# the three audited revisions (66910c10 the corpus clone, 046accf6 the system
# clone, the 90978ac6 slice export), then check both crates' locks at each.
# Run from anywhere; each step's exit is captured to its own file.
# usage: pin.sh BINARY LOG_DIR [--check-only]
set -u
binary="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
logs="$2"
mode="${3:-}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
roots=(
  /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
  /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
)
tags=(66910c10 046accf6 90978ac6)
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
monkey=(--function mk_event_add --function mk_event_timeout_create --function mk_event_channel_create
        --function mk_event_del --function mk_event_timeout_destroy --function mk_event_channel_destroy
        --function mk_event_wait)
setters=(--function flb_input_set_collector_time --function flb_input_set_collector_event
         --function flb_input_set_collector_socket)
if [ "$mode" != "--check-only" ]; then
  for i in 0 1 2; do
    step "pin-libevent-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate qcore-provider-libevent "${monkey[@]}"
    step "pin-fluent-bit-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate quirin-provider-fluent-bit "${setters[@]}"
  done
fi
for i in 0 1 2; do
  step "check-libevent-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate qcore-provider-libevent --check
  step "check-fluent-bit-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate quirin-provider-fluent-bit --check
done
step verify-contracts "$binary" pin --verify-contracts
