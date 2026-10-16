#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [check-only] -- NONNULL-MEMBER-UNIFIED-QUALIFIER
# (adapted from LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER and
# FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE): re-pin, with `quirin-audit pin --bounds`
# at both audited revisions (66910c10, the read-only corpus tree; 046accf6, the
# prepared system tree, read only), every null_deref.qv declaration whose text
# this card changed: in librdkafka the 67 migrated stable/scope declarations
# and the 4 plain declarations that gain the premise (71), in Fluent Bit the
# 14 migrated declarations. Then check each crate's bounds lock at both
# revisions, the whole tree at both, and verify the contracts. Each step's
# output and exit go to their own files.
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
# The declarations, read from the migrated modules: every declaration stating
# a member clause of any kind (librdkafka 72 - rd_kafka_topic_new0 counted
# once = 71; Fluent Bit 14).
functions() {
  python3 - "$1" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
names = [name for name, body in re.findall(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", text, re.S)
         if "requires nonnull_member(" in body]
print(" ".join(names))
PY
}
rk=(); for f in $(functions providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv); do rk+=(--function "$f"); done
fb=(); for f in $(functions providers/quirin-provider-fluent-bit/contracts/fluent_bit/null_deref.qv); do fb+=(--function "$f"); done
[ "${#rk[@]}" = 142 ] || { echo "expected 71 librdkafka declarations, got $(( ${#rk[@]} / 2 ))"; exit 2; }
[ "${#fb[@]}" = 28 ] || { echo "expected 14 Fluent Bit declarations, got $(( ${#fb[@]} / 2 ))"; exit 2; }
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    step "pin-librdkafka-$tag" "$binary" pin --bounds --source-root "$root" --crate qcore-provider-librdkafka "${rk[@]}"
    step "pin-fluent-bit-$tag" "$binary" pin --bounds --source-root "$root" --crate quirin-provider-fluent-bit "${fb[@]}"
  done
fi
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  for crate in qcore-provider-librdkafka quirin-provider-fluent-bit; do
    step "check-bounds-$crate-$tag" "$binary" pin --bounds --source-root "$root" --crate "$crate" --check
  done
  step "check-bounds-all-$tag" "$binary" pin --bounds --source-root "$root" --check
  # The whole-tree resource check also reports the libevent crate's tip-mode
  # `mk_vhost_set` contract, not in force at a Fluent Bit tree by design.
  step "check-all-$tag" "$binary" pin --source-root "$root" --check
done
step verify "$binary" pin --verify-contracts
