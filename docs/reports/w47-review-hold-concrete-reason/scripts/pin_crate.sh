#!/usr/bin/env bash
# pin_crate.sh OUT_DIR -- whole-crate `pin --check` of quirin-provider-fluent-bit
# at the three trees, and `pin --verify-contracts`. Exits to files.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
binary="$repo/target/release/quirin-audit"
mkdir -p "$out"
cd "$repo" || exit 2
declare -A roots=(
  [66910c10]=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
  [046accf6]=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
  [90978ac6]=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
)
for name in 66910c10 046accf6 90978ac6; do
  "$binary" pin --source-root "${roots[$name]}" --crate quirin-provider-fluent-bit --check \
    > "$out/crate-$name.log" 2>&1
  echo $? > "$out/crate-$name.exit"
done
"$binary" pin --verify-contracts > "$out/verify-contracts.log" 2>&1
echo $? > "$out/verify-contracts.exit"
