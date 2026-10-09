#!/usr/bin/env bash
# pin.sh BINARY LOG_DIR [pin|check-only] -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS
# (from VERDICT-CONTRACT-DEPENDENCIES's pin.sh): record the declarations whose
# text gained a premise with `quirin-audit pin` at every revision each was
# reviewed at before this card, then check every changed crate's lock and the
# whole tree at each, and verify the contracts. Each step's output and exit
# go to their own files.
#
# Revisions: 66910c10 (the read-only corpus tree) and 046accf6 (the prepared
# system tree, read only) for every declaration; and, for
# `flb_input_collector_resume` alone, 34155955 (the tail-pause slice's
# export), where it was also reviewed (W47-LOOP-OWNER-CONTRACTS). `pin` starts
# a declaration whose text changed again from the pinned revision alone, so
# omitting that revision would drop the slice's identity and take the
# declaration out of force there. No other changed declaration was reviewed
# at 34155955, and none is pinned there now. The libc declarations are
# flexible: `pin` refreshes their contract digest only.
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
pins="${PINS:-quirin-provider-fluent-bit:flb_output_task_singleplex_enqueue quirin-provider-fluent-bit:flb_input_collector_resume quirin-provider-fluent-bit:flb_input_chunk_get_name qcore-provider-librdkafka:cJSON_New_Item qcore-provider-librdkafka:add_item_to_array qcore-provider-librdkafka:rd_kafka_toppar_new0 qcore-provider-librdkafka:rd_kafka_toppar_desired_add qcore-provider-librdkafka:rd_kafka_q_new0 qcore-provider-librdkafka:ut_create_mock_queue quirin-provider-libc:__errno_location quirin-provider-libc:strerror}"
slice_pins="${SLICE_PINS:-quirin-provider-fluent-bit:flb_input_collector_resume}"
crates=$(for pin in $pins; do echo "${pin%%:*}"; done | sort -u | tr '\n' ' ')
pin_at() {
  local root="$1" tag="$2" list="$3" crate functions
  for crate in $(for pin in $list; do echo "${pin%%:*}"; done | sort -u); do
    functions=()
    for pin in $list; do
      [ "${pin%%:*}" = "$crate" ] && functions+=(--function "${pin#*:}")
    done
    step "pin-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" "${functions[@]}"
  done
}
if [ "$mode" = pin ]; then
  pin_at "$old" 66910c10 "$pins"
  pin_at "$new" 046accf6 "$pins"
  pin_at "$slice" 34155955 "$slice_pins"
fi
for pair in "$old:66910c10" "$new:046accf6" "$slice:34155955"; do
  root="${pair%:*}"; tag="${pair##*:}"
  step "check-$tag" "$binary" pin --source-root "$root" --check
  for crate in $crates; do
    step "check-$crate-$tag" "$binary" pin --source-root "$root" --crate "$crate" --check
  done
done
step verify "$binary" pin --verify-contracts
