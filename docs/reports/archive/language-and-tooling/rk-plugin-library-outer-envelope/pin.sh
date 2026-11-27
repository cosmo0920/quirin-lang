#!/usr/bin/env bash
# Re-pin every changed provider declaration at both audited source revisions.
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
roots="${4:-both}"
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
  [ "$(cat "$logs/$name.exit")" = 0 ]
}
functions() {
  python3 - "$@" <<'PY'
import re, sys
for path in sys.argv[1:]:
    source = open(path, encoding="utf-8").read()
    for name, body in re.findall(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", source, re.S):
        if "requires resource_premise(rk_plugin_callback_closure" in body:
            print(name)
PY
}
rk_resource=()
for f in $(functions providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv providers/qcore-provider-librdkafka/contracts/librdkafka/kafka_mem.qv); do rk_resource+=(--function "$f"); done
rk_bounds=()
for f in $(functions providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv); do rk_bounds+=(--function "$f"); done
fb_resource=()
for f in $(functions providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv); do fb_resource+=(--function "$f"); done
[ "${#rk_resource[@]}" = 182 ] || { echo "expected 91 librdkafka resource declarations"; exit 2; }
[ "${#rk_bounds[@]}" = 142 ] || { echo "expected 71 librdkafka bounds declarations"; exit 2; }
[ "${#fb_resource[@]}" = 6 ] || { echo "expected 3 Fluent Bit resource declarations"; exit 2; }
if [ "$roots" = second ]; then roots_to_pin=("$new"); else roots_to_pin=("$old" "$new"); fi
for root in "${roots_to_pin[@]}"; do
  tag="$(basename "$(dirname "$root")")"
  if [ "$mode" = pin ]; then
    step "rk-resource-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-librdkafka "${rk_resource[@]}" || exit 3
    step "rk-bounds-$tag" "$binary" pin --bounds --source-root "$root" --crate qcore-provider-librdkafka "${rk_bounds[@]}" || exit 3
    step "fb-resource-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-fluent-bit "${fb_resource[@]}" || exit 3
    step "rk-outer-$tag" "$binary" pin --outer --source-root "$root" --crate qcore-provider-librdkafka || exit 3
    step "fb-outer-$tag" "$binary" pin --outer --source-root "$root" --crate quirin-provider-fluent-bit || exit 3
  fi
  step "check-rk-resource-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-librdkafka --check || exit 4
  step "check-rk-bounds-$tag" "$binary" pin --bounds --source-root "$root" --crate qcore-provider-librdkafka --check || exit 4
  step "check-fb-resource-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-fluent-bit --check "${fb_resource[@]}" || exit 4
  step "check-rk-outer-$tag" "$binary" pin --outer --source-root "$root" --crate qcore-provider-librdkafka --check || exit 4
  step "check-fb-outer-$tag" "$binary" pin --outer --source-root "$root" --crate quirin-provider-fluent-bit --check || exit 4
done
step verify "$binary" pin --verify-contracts
