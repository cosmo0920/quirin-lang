#!/bin/sh
# pin.sh BINARY LOG_DIR [check] -- FB-RETRY-LIFECYCLE-CONTRACTS: record the
# eight reviewed retry_lifecycle.qv declarations in quirin-provider-fluent-bit's
# contracts.lock with `quirin-audit pin` at 046accf6 (this host's prepared
# system tree, read only; the 66910c10 corpus tree is not on this host, so no
# identity is recorded there), then check the crate's lock at 046accf6 and
# verify the contracts. Each step's output and exit go to their own files.
BIN=$1
LOGS=$2
MODE=${3:-pin}
ROOT=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6/fluent-bit-clang-source
REPO=$(cd "$(dirname "$0")/../../../.." && pwd)
mkdir -p "$LOGS"
cd "$REPO" || exit 2
step() {
  name=$1; shift
  "$@" > "$LOGS/$name.log" 2>&1
  echo $? > "$LOGS/$name.exit"
  echo "$name $(cat "$LOGS/$name.exit")"
}
if [ "$MODE" = pin ]; then
  step pin-046accf6 "$BIN" pin --source-root "$ROOT" --crate quirin-provider-fluent-bit \
    --function flb_sched_timer_cb_disable --function map_free_task_id \
    --function flb_task_from_fs_storage --function flb_sched_ctx_init \
    --function flb_output_thread_instance_get --function flb_metrics_sum \
    --function flb_output_name --function flb_input_name
fi
step check-fluent-bit-046accf6 "$BIN" pin --source-root "$ROOT" --crate quirin-provider-fluent-bit --check
step verify "$BIN" pin --verify-contracts
