#!/usr/bin/env bash
# OP-APP-RK-CALLBACKS-VIA-POLL-CB (adapted from OP-CALLBACK-CANDIDATE-CONTRACTS, from OP-CONSTRUCTOR-DECLARATIONS, from CG22-OP-CALLBACKS-AND-SERVED-QUEUES, from CG22-CALLBACK-REGISTRY-CONTRACT, adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES, adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1) (from W45-CALLEE-CASCADE-RESIDUAL.s and
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
# PINS: "crate:function ..." per stage, from the environment (default: stage
# (a)'s declarations).
# OP-APP-RK-CALLBACKS-VIA-POLL-CB: its declarations, and the nineteen op-callback
# candidates whose premise it restated.
pins="${PINS:-qcore-provider-librdkafka:rd_kafka_broker_monitor_op_cb qcore-provider-librdkafka:rd_kafka_cgrp_assignor_handle_Metadata_op qcore-provider-librdkafka:rd_kafka_cgrp_consumer_handle_Metadata_op qcore-provider-librdkafka:rd_kafka_cgrp_handle_Metadata_op qcore-provider-librdkafka:rd_kafka_offset_reset_op_cb qcore-provider-librdkafka:rd_kafka_offset_validate_op_cb qcore-provider-librdkafka:rd_kafka_share_ack_commit_cb_register_op qcore-provider-librdkafka:rd_kafka_share_fetch_reply_op_cb qcore-provider-librdkafka:rd_kafka_txn_op_init_transactions qcore-provider-librdkafka:rd_kafka_txn_op_ack_init_transactions qcore-provider-librdkafka:rd_kafka_txn_op_begin_transaction qcore-provider-librdkafka:rd_kafka_txn_op_begin_commit qcore-provider-librdkafka:rd_kafka_txn_op_commit_transaction qcore-provider-librdkafka:rd_kafka_txn_op_commit_transaction_ack qcore-provider-librdkafka:rd_kafka_txn_op_begin_abort qcore-provider-librdkafka:rd_kafka_txn_op_abort_transaction qcore-provider-librdkafka:rd_kafka_txn_op_abort_transaction_ack qcore-provider-librdkafka:rd_kafka_cgrp_op_serve qcore-provider-librdkafka:rd_kafka_mock_cluster_op_serve qcore-provider-librdkafka:rd_kafka_conf_set_background_event_cb qcore-provider-librdkafka:rd_kafka_conf_set_dr_cb qcore-provider-librdkafka:rd_kafka_conf_set_dr_msg_cb qcore-provider-librdkafka:rd_kafka_conf_set_consume_cb qcore-provider-librdkafka:rd_kafka_conf_set_rebalance_cb qcore-provider-librdkafka:rd_kafka_conf_set_offset_commit_cb qcore-provider-librdkafka:rd_kafka_conf_set_error_cb qcore-provider-librdkafka:rd_kafka_conf_set_throttle_cb qcore-provider-librdkafka:rd_kafka_conf_set_log_cb qcore-provider-librdkafka:rd_kafka_conf_set_stats_cb qcore-provider-librdkafka:rd_kafka_conf_set_oauthbearer_token_refresh_cb qcore-provider-librdkafka:rd_kafka_conf_set_socket_cb qcore-provider-librdkafka:rd_kafka_conf_set_connect_cb qcore-provider-librdkafka:rd_kafka_conf_set_closesocket_cb qcore-provider-librdkafka:rd_kafka_conf_set_open_cb qcore-provider-librdkafka:rd_kafka_conf_set_resolve_cb qcore-provider-librdkafka:rd_kafka_conf_set_ssl_cert_verify_cb qcore-provider-librdkafka:rd_kafka_conf_new qcore-provider-librdkafka:rd_kafka_consume_cb qcore-provider-librdkafka:rd_kafka_background_queue_serve qcore-provider-librdkafka:rd_kafka_oauthbearer_refresh_op qcore-provider-librdkafka:rd_kafka_log_print qcore-provider-librdkafka:rd_kafka_oauthbearer_unsecured_token qcore-provider-librdkafka:rd_kafka_oidc_token_jwt_bearer_refresh_cb qcore-provider-librdkafka:rd_kafka_oidc_token_client_credentials_refresh_cb qcore-provider-librdkafka:rd_kafka_oidc_token_metadata_azure_imds_refresh_cb qcore-provider-librdkafka:rd_kafka_oidc_token_metadata_aws_iam_refresh_cb quirin-provider-fluent-bit:cb_kafka_msg quirin-provider-fluent-bit:cb_kafka_logger quirin-provider-fluent-bit:oauthbearer_token_refresh_cb}"
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
