#!/usr/bin/env bash
# probe.sh -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: dump the three
# librdkafka units' functions (unit_ast.py, unit_functions.py into
# private/mpuh/) and run the member-path probe on the eight declarations,
# output to artifacts/probe.txt. A reading aid naming each reason; the
# arbiter's sidecar is the published standing.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpuh"
cd "$repo" || exit 2
contracts=providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv
for unit in rdkafka_partition rdkafka_admin rdkafka; do
  if [ ! -f "$out/$unit.functions.json" ]; then
    python3 "$here/unit_ast.py" "lib/librdkafka-2.15.0/src/$unit.c" "$out/$unit.json" || exit 1
    python3 "$here/unit_functions.py" "$out/$unit.json" "$out/$unit.functions.json" || exit 1
    rm -f "$out/$unit.json"
  fi
done
probe() {
  cargo run -q -p qcore-c-gateway --example member_path_probe -- "$contracts" "$out/$1.functions.json" rd_kafka_op_s "${@:2}"
}
{
  probe rdkafka_partition rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb
  probe rdkafka_admin rd_kafka_admin_worker rd_kafka_admin_fanout_worker \
    rd_kafka_DeleteRecords_leaders_queried_cb rd_kafka_ListOffsets_leaders_queried_cb
  probe rdkafka rd_kafka_share_commit_async_fanout_op rd_kafka_share_fetch_fanout_op \
    rd_kafka_share_commit_sync_fanout_op
} > "$here/../artifacts/probe.txt"
echo $? > "$out/probe.exit"
