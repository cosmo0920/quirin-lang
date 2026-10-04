#!/usr/bin/env bash
# CALLEE-RETENTION-CLASS1 (from CG10-INDIRECT-ACCESS-SOLE-RESIDUALS's
# pin.sh): record the reviewed declarations with `quirin-audit pin` at both
# audited revisions (66910c10, the read-only corpus tree; 046accf6, the
# prepared system tree, read only), then check every changed crate's lock at
# both, the whole tree at both, and verify the contracts. Each step's output
# and exit go to their own files.
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
crates="qcore-provider-librdkafka qcore-provider-ctraces quirin-provider-cfl qcore-provider-chunkio"
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    step "pin-librdkafka-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-librdkafka \
      --function cJSON_New_Item --function LZ4_compress_HC --function LZ4_decompress_unsafe_generic \
      --function LZ4_compress_default --function load_descriptor_values \
      --function rd_kafka_aborted_txns_next_offset --function rd_kafka_anyconf_desensitize \
      --function rd_kafka_dump0 --function rd_kafka_metadata_copy_internal
    step "pin-ctraces-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-ctraces \
      --function ctr_mpack_consume_uint_tag --function ctr_mpack_consume_int_tag \
      --function ctr_attributes_set_string --function ctr_attributes_set_bool \
      --function ctr_attributes_set_int64 --function ctr_attributes_set_double
    step "pin-cfl-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-cfl \
      --function cfl_kvlist_count --function cfl_kvlist_insert_string --function cfl_kvlist_insert_bool \
      --function cfl_kvlist_insert_int64 --function cfl_kvlist_insert_double
    step "pin-chunkio-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-chunkio \
      --function _cio_file_up
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  # The whole-tree check also reports the libevent crate's tip-mode
  # `mk_vhost_set` contract, not in force at a Fluent Bit tree by design
  # (MONKEY-VHOST-PROPERTY-CUSTODY); the crates this card changes are checked
  # on their own as well.
  step "check-$tag" "$binary" pin --source-root "$root" --check
  for crate in $crates; do
    step "check-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" --check
  done
done
step verify "$binary" pin --verify-contracts
