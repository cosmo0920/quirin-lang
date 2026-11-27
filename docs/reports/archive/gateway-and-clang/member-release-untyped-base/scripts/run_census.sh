#!/bin/sh
# MEMBER-RELEASE-UNTYPED-BASE: the census command, over the inputs
# MEMBER-EFFECT-COMPONENT-SCOPE's census read (baseline:7b726587b reports, the
# source trees those runs analysed, the 046accf6 premises census).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../../.." && pwd)
MAIN=/media/Data/GitHub/quirin-lang
B=$MAIN/private/baselines/7b726587bdf6a8213f451fa1c78297bb5e476ef7
OUT=$HERE/../census
mkdir -p "$OUT"
python3 "$HERE/census.py" report corpus "$B/eight-home/corpus/corpus/resource-diagnostics.json" \
  "$MAIN/private/measure/member-effect-20261105T185335Z/mirror/c-gateway-wave-pin" \
  "$REPO/providers" > "$OUT/corpus.json"
python3 "$HERE/census.py" report system "$B/fluent-bit-system/system/resource-diagnostics.json" \
  "$MAIN/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source" \
  "$REPO/providers" > "$OUT/system.json"
python3 "$HERE/census.py" report slice "$B/fluent-bit-tail-pause/slice/resource-diagnostics.json" \
  "$MAIN/private/w47-tail/attempt-1/prep/fluent-bit-clang-source" \
  "$REPO/providers" > "$OUT/slice.json"
python3 "$HERE/census.py" current "$REPO/docs/reports/member-access-projection/census/premises.jsonl.gz" \
  "$MAIN/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source" \
  "$REPO/providers" > "$OUT/current.json"
