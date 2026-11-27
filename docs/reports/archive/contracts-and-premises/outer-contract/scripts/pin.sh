#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [check-only] -- OUTER-CONTRACT (from
# FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE's pin.sh): pin the Fluent Bit outer
# contract's loader declarations (flb_plugin_load, flb_plugin_proxy_create)
# with `quirin-audit pin --outer` at both audited revisions (66910c10, the
# read-only corpus tree; 046accf6, the prepared system tree, read only), then
# `pin --outer --check` at both and verify every lock. Each step's output and
# exit go to their own files.
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
    step "pin-$tag" "$binary" pin --outer --source-root "$root" --crate quirin-provider-fluent-bit
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "check-$tag" "$binary" pin --outer --source-root "$root" --check
done
step verify "$binary" pin --verify-contracts
