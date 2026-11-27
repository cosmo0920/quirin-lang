#!/usr/bin/env bash
# W47-CALLEE-CLOSURE-R2: pin the two reviewed declarations at both audited
# revisions, then check every crate's lock at both. Run from the repository
# root; each step's exit is captured to its own file.
# usage: pin.sh BINARY LOG_DIR
set -u
binary="$1"
logs="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w47-callee-closure-r2/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "pin-cfl-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-cfl --function cfl_arena_calloc
  step "pin-rdkafka-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-librdkafka --function rd_file_read
done
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "check-$tag" "$binary" pin --source-root "$root" --check
done
