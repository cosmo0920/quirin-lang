#!/usr/bin/env bash
# CG22-CALLBACK-REGISTRY-CONTRACT (adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES, adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1) (from W45-CALLEE-CASCADE-RESIDUAL.s and
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
# REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: this card's ten new declarations
# and the re-declared `rd_kafka_queue_new0`.
pins="${PINS:-qcore-provider-librdkafka:rd_kafka_toppar_get0 qcore-provider-librdkafka:rd_kafka_toppar_desired_get qcore-provider-librdkafka:rd_kafka_toppar_new0 qcore-provider-librdkafka:rd_kafka_toppar_desired_add qcore-provider-librdkafka:rd_kafka_toppar_get2 qcore-provider-librdkafka:rd_kafka_toppar_get_avail qcore-provider-librdkafka:rd_kafka_q_new0 qcore-provider-librdkafka:ut_create_mock_toppar qcore-provider-librdkafka:ut_create_mock_queue qcore-provider-librdkafka:ut_destroy_mock_queue qcore-provider-librdkafka:rd_kafka_queue_new0}"
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
