#!/usr/bin/env bash
# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (from W48-CALLEE-CLOSURE-R3's pin.sh):
# record the reviewed declarations with `quirin-audit pin` at both audited
# revisions (66910c10, the read-only corpus tree; 046accf6, the prepared
# system tree, read only), then check every crate's lock at both and verify
# the contracts. Each step's output and exit go to their own files.
# usage: pin.sh BINARY LOG_DIR [check-only]
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
    step "pin-cmetrics-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-cmetrics \
      --function cmt_atomic_load --function cmt_atomic_store --function cmt_atomic_compare_exchange
    step "pin-librdkafka-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-librdkafka \
      --function rd_atomic64_get --function rd_atomic32_add --function rd_atomic64_add \
      --function rd_atomic32_set --function cnd_timedwait_ms --function rd_hdr_histogram_record
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "check-$tag" "$binary" pin --source-root "$root" --check
  # The whole-tree check also reports the libevent crate's tip-mode
  # `mk_vhost_set` contract, not in force at a Fluent Bit tree by design
  # (MONKEY-VHOST-PROPERTY-CUSTODY); the two crates this card changes are
  # checked on their own as well.
  for crate in qcore-provider-cmetrics qcore-provider-librdkafka; do
    step "check-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" --check
  done
done
step verify "$binary" pin --verify-contracts
