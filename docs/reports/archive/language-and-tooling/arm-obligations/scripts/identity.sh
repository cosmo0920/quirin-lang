#!/bin/sh
# ARM-OBLIGATIONS: the identity checks of BASE -> TIP, written to OUT_DIR.
# 1. strip_carriage.py checks and undoes this card's corpus row additions;
# 2. RULE36-PILOT's corpus_substitution.py and byte comparisons, and
#    RULE35-PILOT's strict token substitution, run unchanged on the result.
# Usage: identity.sh WORKTREE MEASURE_DIR OUT_DIR   (OUT_DIR must not exist)
set -u
W=$1
M=$2/sides
O=$3
cd "$W" || exit 2
mkdir "$O" || exit 2
R=docs/reports/rule36-pilot/scripts
S=docs/reports/arm-obligations/scripts
python3 $S/strip_carriage.py "$M/base/corpus" "$M/tip/corpus" "$O/tip-stripped/corpus" >"$O/strip.txt"
echo $? >"$O/strip.exit"
T=$O/tip-stripped/corpus
B=$M/base/corpus
python3 $R/compare_bytes.py "$M/base/system" "$M/tip/system" >"$O/bytes-system.txt"
python3 $R/compare_bytes.py "$M/base/slice" "$M/tip/slice" >"$O/bytes-slice.txt"
python3 $R/compare_bytes.py "$B/corpus" "$M/tip/corpus/corpus" >"$O/bytes-corpus.txt"
python3 $R/corpus_substitution.py "$B/corpus" "$T/corpus" "$M/base" "$M/tip" >"$O/corpus-substitution.txt"
echo $? >"$O/corpus-substitution.exit"
python3 docs/reports/rule35-pilot/scripts/substitution_ids.py \
  "$B/corpus/resource-diagnostics.json" "$T/corpus/resource-diagnostics.json" \
  "$B/corpus/resource-diagnostics.components.json" "$T/corpus/resource-diagnostics.components.json" \
  "$B/corpus/resource-diagnostics.contracts.json" "$T/corpus/resource-diagnostics.contracts.json" \
  "$B/corpus/resource-diagnostics.source-effects.json" "$T/corpus/resource-diagnostics.source-effects.json" \
  "$B/corpus/resource-diagnostics.chunk-contracts.json" "$T/corpus/resource-diagnostics.chunk-contracts.json" \
  "$B/corpus-null.jsonl" "$T/corpus-null.jsonl" \
  "$B/corpus-null.summary.json" "$T/corpus-null.summary.json" \
  "$B/corpus-outer.json" "$T/corpus-outer.json" >"$O/substitution-ids.jsonl"
echo $? >"$O/substitution-ids.exit"
