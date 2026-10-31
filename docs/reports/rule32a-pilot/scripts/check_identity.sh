#!/bin/sh
# RULE32A-PILOT: the by-hand identity checks of BASE (the published
# baseline:db2795100) -> TIP (the measured `tip` side), written to OUT_DIR:
# byte comparison of every route's artifacts (RULE36-PILOT's compare_bytes.py),
# RULE36-PILOT's corpus substitution check and RULE35-PILOT's strict token
# substitution (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY).
# Usage: check_identity.sh WORKTREE MEASURE_DIR OUT_DIR
set -u
W=$1
T=$2/sides/tip
O=$3
B=/media/Data/GitHub/quirin-lang/private/baselines/db2795100a760efa0a2cbcbe13217bffc3cf4733
cd "$W" || exit 2
S=docs/reports/rule36-pilot/scripts
python3 $S/compare_bytes.py "$B/fluent-bit-system/system" "$T/system" >"$O/bytes-system.txt"
python3 $S/compare_bytes.py "$B/fluent-bit-tail-pause/slice" "$T/slice" >"$O/bytes-slice.txt"
python3 $S/compare_bytes.py "$B/eight-home/corpus/corpus" "$T/corpus/corpus" >"$O/bytes-corpus.txt"
python3 $S/compare_bytes.py "$B/eight-home/corpus" "$T/corpus" >"$O/bytes-corpus-top.txt"
python3 $S/corpus_substitution.py "$B/eight-home/corpus/corpus" "$T/corpus/corpus" \
  "$B/eight-home" "$T" >"$O/corpus-substitution.txt"
echo $? >"$O/corpus-substitution.exit"
BC=$B/eight-home/corpus
TC=$T/corpus
python3 docs/reports/rule35-pilot/scripts/substitution_ids.py \
  "$BC/corpus/resource-diagnostics.json" "$TC/corpus/resource-diagnostics.json" \
  "$BC/corpus/resource-diagnostics.components.json" "$TC/corpus/resource-diagnostics.components.json" \
  "$BC/corpus/resource-diagnostics.contracts.json" "$TC/corpus/resource-diagnostics.contracts.json" \
  "$BC/corpus/resource-diagnostics.source-effects.json" "$TC/corpus/resource-diagnostics.source-effects.json" \
  "$BC/corpus/resource-diagnostics.chunk-contracts.json" "$TC/corpus/resource-diagnostics.chunk-contracts.json" \
  "$BC/corpus-null.jsonl" "$TC/corpus-null.jsonl" \
  "$BC/corpus-null.summary.json" "$TC/corpus-null.summary.json" \
  "$BC/corpus-outer.json" "$TC/corpus-outer.json" >"$O/substitution-ids.jsonl"
echo $? >"$O/substitution-ids.exit"
