#!/usr/bin/env bash
# rank-all.sh NAME -- REFCOUNT-FAMILY-BLOCKING-CALLEES (adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1): scripts/rank.py over the run
# NAME's corpus, system and tail-pause slice reports into
# artifacts/rank-NAME-{corpus,system,slice}.json, then scripts/tally.py over the
# corpus ranking into artifacts/tally-NAME.txt. Exits and logs go to
# private/rfbc/rank-NAME-*.{log,exit}; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rfbc"
art="$here/../artifacts"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/rfbc-$name/resource-diagnostics.json
mkdir -p "$art"
python3 "$here/rank.py" "$out/$name/resource-diagnostics.json" "$art/rank-$name-corpus.json" \
  --source-root "$old" > "$out/rank-$name-corpus.log" 2>&1
echo $? > "$out/rank-$name-corpus.exit"
python3 "$here/rank.py" "$out/$name-system/resource-diagnostics.json" "$art/rank-$name-system.json" \
  > "$out/rank-$name-system.log" 2>&1
echo $? > "$out/rank-$name-system.exit"
python3 "$here/rank.py" "$slice" "$art/rank-$name-slice.json" > "$out/rank-$name-slice.log" 2>&1
echo $? > "$out/rank-$name-slice.exit"
python3 "$here/tally.py" "$art/rank-$name-corpus.json" "$art/classification-w45.json" > "$art/tally-$name.txt" 2> "$out/tally-$name.log"
echo $? > "$out/tally-$name.exit"
echo "corpus $(cat "$out/rank-$name-corpus.exit") system $(cat "$out/rank-$name-system.exit") slice $(cat "$out/rank-$name-slice.exit") tally $(cat "$out/tally-$name.exit")"
