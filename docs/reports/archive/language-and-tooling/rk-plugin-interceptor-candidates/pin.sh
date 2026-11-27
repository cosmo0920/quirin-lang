#!/usr/bin/env bash
set -u
binary="$1"
out="$2"
mkdir -p "$out"
failed=0
step() {
  local name="$1"; shift
  "$@" > "$out/$name.log" 2>&1
  local status=$?
  echo "$status" > "$out/$name.exit"
  echo "$name $status"
  if [ "$status" != 0 ]; then failed=1; fi
}
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
for pair in old new; do
  if [ "$pair" = old ]; then root="$old"; else root="$new"; fi
  for crate in qcore-provider-librdkafka quirin-provider-fluent-bit; do
    step "pin-$crate-$pair" "$binary" pin --outer --source-root "$root" --crate "$crate"
    step "check-$crate-$pair" "$binary" pin --outer --source-root "$root" --crate "$crate" --check
  done
done
step verify "$binary" pin --verify-contracts
exit "$failed"
