#!/usr/bin/env bash
# W48-CALLEE-CLOSURE-R3: record the reviewed libc declarations with
# `quirin-audit pin` at both audited revisions, then check every crate's lock
# at both (adapted from w47-callee-closure-r2/scripts/pin.sh). The libc
# declarations are `revision flexible`, so the lock records their contract
# digests only. Run from anywhere; each step's exit goes to its own file.
# usage: pin.sh BINARY LOG_DIR [check-only]
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w48-callee-closure-r3/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    step "pin-libc-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-libc \
      --function lstat --function clock_gettime
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "check-$tag" "$binary" pin --source-root "$root" --check
done
