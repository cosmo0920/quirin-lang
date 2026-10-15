#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [check-only] -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED
# (adapted from OP-CALLBACK-MEMBER-PATH-RELEASE, from OP-APP-RK-CALLBACKS-VIA-POLL-CB
# and its predecessors): record the reviewed declarations with `quirin-audit
# pin` at both audited revisions (66910c10, the read-only corpus tree;
# 046accf6, the prepared system tree, read only), then check every changed
# crate's lock at both, the whole tree at both, and verify the contracts. Each
# step's output and exit go to their own files.
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
# PINS: "crate:function ..." from the environment (default: the 34 client.qv
# declarations write_contracts.py gave the premise).
pins="${PINS:-qcore-provider-librdkafka:rd_kafka_err_action qcore-provider-librdkafka:rd_kafka_oauthbearer_set_token_failure qcore-provider-librdkafka:rd_kafka_toppars_pause_resume qcore-provider-librdkafka:rd_kafka_toppar_destroy0 qcore-provider-librdkafka:rd_kafka_q_destroy0 qcore-provider-librdkafka:rd_kafka_q_destroy_owner qcore-provider-librdkafka:rd_kafka_topic_destroy0 qcore-provider-librdkafka:rd_kafka_enq_once_trigger qcore-provider-librdkafka:rd_kafka_topic_partition_list_destroy qcore-provider-librdkafka:rd_kafka_topic_partition_destroy0 qcore-provider-librdkafka:rd_kafka_AlterConsumerGroupOffsets_destroy qcore-provider-librdkafka:rd_kafka_DeleteConsumerGroupOffsets_destroy qcore-provider-librdkafka:rd_kafka_ListConsumerGroupOffsets_destroy qcore-provider-librdkafka:rd_kafka_topic_partition_result_destroy qcore-provider-librdkafka:rd_kafka_toppar_new0 qcore-provider-librdkafka:rd_kafka_toppar_desired_add qcore-provider-librdkafka:rd_kafka_toppar_get2 qcore-provider-librdkafka:rd_kafka_toppar_get_avail qcore-provider-librdkafka:ut_create_mock_toppar qcore-provider-librdkafka:ut_destroy_mock_queue qcore-provider-librdkafka:rd_kafka_q_wait_result qcore-provider-librdkafka:rd_kafka_q_purge0 qcore-provider-librdkafka:rd_kafka_toppar_fetch_backoff qcore-provider-librdkafka:unit_test_telemetry qcore-provider-librdkafka:rd_kafka_q_serve0 qcore-provider-librdkafka:rd_kafka_q_serve qcore-provider-librdkafka:rd_kafka_q_serve_maybe_consume qcore-provider-librdkafka:rd_kafka_q_pop_serve0 qcore-provider-librdkafka:rd_kafka_q_pop_serve qcore-provider-librdkafka:rd_kafka_q_pop_serve_maybe_consume qcore-provider-librdkafka:rd_kafka_op_destroy qcore-provider-librdkafka:rd_kafka_op_req0 qcore-provider-librdkafka:rd_kafka_op_req qcore-provider-librdkafka:rd_kafka_op_req2}"
crates=$(for pin in $pins; do echo "${pin%%:*}"; done | sort -u | tr '\n' ' ')
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    for crate in $crates; do
      functions=()
      for pin in $pins; do
        [ "${pin%%:*}" = "$crate" ] && functions+=(--function "${pin#*:}")
      done
      step "pin-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" "${functions[@]}"
    done
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
