#!/bin/sh
# Pilot round 2: the identity checks of the shared measurement's BASE -> TIP,
# written to OUT_DIR: byte comparison of every route's artifacts, this card's
# corpus substitution check, and RULE35-PILOT's strict token substitution.
# Usage: r2_identity.sh WORKTREE MEASURE_DIR OUT_DIR
set -u
W=$1
M=$2/sides
O=$3
cd "$W" || exit 2
S=docs/reports/rule36-pilot/scripts
python3 $S/compare_bytes.py "$M/base/system" "$M/tip/system" >"$O/bytes-system.txt"
python3 $S/compare_bytes.py "$M/base/slice" "$M/tip/slice" >"$O/bytes-slice.txt"
python3 $S/compare_bytes.py "$M/base/corpus/corpus" "$M/tip/corpus/corpus" >"$O/bytes-corpus.txt"
python3 $S/compare_bytes.py "$M/base/corpus" "$M/tip/corpus" >"$O/bytes-corpus-top.txt"
python3 $S/corpus_substitution.py "$M/base/corpus/corpus" "$M/tip/corpus/corpus" "$M/base" "$M/tip" >"$O/corpus-substitution.txt"
echo $? >"$O/corpus-substitution.exit"
B=$M/base/corpus
T=$M/tip/corpus
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
