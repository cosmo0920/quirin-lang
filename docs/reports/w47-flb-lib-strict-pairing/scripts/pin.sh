#!/usr/bin/env bash
# pin.sh LOG_DIR -- re-pin the three changed lifecycle declarations
# (flb_start, flb_stop, flb_destroy) of quirin-provider-fluent-bit at the
# three reviewed trees, then `pin --check` of those three at each (card
# W47-FLB-LIB-STRICT-PAIRING). `pin` only reads the source roots; it writes
# providers/quirin-provider-fluent-bit/contracts.lock. Every exit to a file.
set -u
logs="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$logs" || exit 2
cd "$repo" || exit 2
corpus=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
system=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
functions=(--function flb_start --function flb_stop --function flb_destroy)
for pair in "66910c10:$corpus" "046accf6:$system" "90978ac6:$slice"; do
  name="${pair%%:*}"; root="${pair#*:}"
  target/release/quirin-audit pin --source-root "$root" --crate quirin-provider-fluent-bit \
    "${functions[@]}" > "$logs/pin-$name.log" 2>&1
  echo $? > "$logs/pin-$name.exit"
done
for pair in "66910c10:$corpus" "046accf6:$system" "90978ac6:$slice"; do
  name="${pair%%:*}"; root="${pair#*:}"
  target/release/quirin-audit pin --source-root "$root" --crate quirin-provider-fluent-bit \
    "${functions[@]}" --check > "$logs/check-$name.log" 2>&1
  echo $? > "$logs/check-$name.exit"
done
