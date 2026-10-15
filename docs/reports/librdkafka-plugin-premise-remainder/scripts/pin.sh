#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [check-only] -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER
# (adapted from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED): record the reviewed
# declarations with `quirin-audit pin` at both audited revisions (66910c10, the
# read-only corpus tree; 046accf6, the prepared system tree, read only): the
# 67 premised null_deref.qv declarations in the bounds lock (`--bounds`) and
# rd_kafka_crash / rd_kafka_dump0 in the resource lock. Then check the crate's
# locks at both revisions, the whole tree at both, and verify the contracts.
# The model contract rd_kafka_q_destroy_final is in no view and no lock. Each
# step's output and exit go to their own files.
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
crate=qcore-provider-librdkafka
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
bounds=()
for f in $(python3 -c '
import json, sys
rows = json.load(open(sys.argv[1]))["rows"]
print(" ".join(sorted({r["function"] for r in rows if r["premise"]})))
' "$here/../artifacts/member-census.json"); do
  bounds+=(--function "$f")
done
[ "${#bounds[@]}" = 134 ] || { echo "expected 67 bounds declarations"; exit 2; }
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    step "pin-bounds-$tag" "$binary" pin --bounds --source-root "$root" --crate "$crate" "${bounds[@]}"
    step "pin-resource-$tag" "$binary" pin --source-root "$root" --crate "$crate" \
      --function rd_kafka_crash --function rd_kafka_dump0
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  # The whole-tree check also reports the libevent crate's tip-mode
  # `mk_vhost_set` contract, not in force at a Fluent Bit tree by design
  # (MONKEY-VHOST-PROPERTY-CUSTODY); the crate this card changes is checked
  # on its own as well.
  step "check-$tag" "$binary" pin --source-root "$root" --check
  step "check-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" --check
  step "check-bounds-$crate-$tag" "$binary" pin --bounds --source-root "$root" --crate "$crate" --check
done
step verify "$binary" pin --verify-contracts
