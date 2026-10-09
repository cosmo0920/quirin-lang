#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [check-only] -- NULL-SAFETY-PROPERTY (adapted from
# CG22-CALLBACK-REGISTRY-CONTRACT's and CHUNKIO-META-LEN-BOUNDS-CONTRACTS's):
# record the reviewed `quant null-deref` declarations with `quirin-audit pin
# --bounds` at both audited revisions (66910c10, the read-only corpus tree;
# 046accf6, the prepared system tree, read only), then `pin --bounds --check`
# every changed crate at both, and verify the contracts. Each step's output
# and exit go to their own files.
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
pins="qcore-provider-librdkafka:rd_kafka_destroy qcore-provider-librdkafka:rd_kafka_topic_partition_list_destroy qcore-provider-librdkafka:rd_kafka_topic_partition_list_add qcore-provider-librdkafka:rd_list_destroy qcore-provider-librdkafka:rd_list_add qcore-provider-librdkafka:rd_kafka_queue_destroy qcore-provider-librdkafka:rd_kafka_message_destroy qcore-provider-cmetrics:cmt_destroy quirin-provider-fluent-bit:flb_output_set_context quirin-provider-fluent-bit:flb_output_net_default quirin-provider-fluent-bit:flb_output_name quirin-provider-fluent-bit:flb_output_set_property quirin-provider-fluent-bit:flb_input_chunk_get_name"
crates=$(for pin in $pins; do echo "${pin%%:*}"; done | sort -u | tr '\n' ' ')
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    for crate in $crates; do
      functions=()
      for pin in $pins; do
        [ "${pin%%:*}" = "$crate" ] && functions+=(--function "${pin#*:}")
      done
      step "pin-$crate-$tag" "$binary" pin --bounds --source-root "$root" --crate "$crate" "${functions[@]}"
    done
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  for crate in $crates; do
    step "check-$crate-$tag" "$binary" pin --bounds --source-root "$root" --crate "$crate" --check
  done
  step "check-all-$tag" "$binary" pin --bounds --source-root "$root" --check
done
step verify "$binary" pin --verify-contracts
