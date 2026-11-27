#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [check-only] -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
# round 2 (from the first round's pin.sh): record the reviewed declarations
# with `quirin-audit pin` at both audited revisions (66910c10, the read-only
# corpus tree; 046accf6, the prepared system tree, read only), then check each
# changed crate's lock and the whole tree at both and verify the contracts.
# Each step's output and exit go to their own files.
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
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
    step "pin-chunkio-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-chunkio \
      --function cio_errno_print
    step "pin-ctraces-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-ctraces \
      --function ctr_errno_print
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "check-$tag" "$binary" pin --source-root "$root" --check
  for crate in qcore-provider-chunkio qcore-provider-ctraces; do
    step "check-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" --check
  done
done
step verify "$binary" pin --verify-contracts
