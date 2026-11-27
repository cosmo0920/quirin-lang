#!/bin/sh
# ARM-OBLIGATIONS-UNIT-GRAIN: the identity checks of BASE -> TIP, written to
# OUT_DIR.
# 1. narrowed_carriage.py checks that TIP only narrows rule 37's carriage and
#    undoes the narrowing on BASE (the dropped rows' fields and digests, the
#    contract sidecar's quotations of them);
# 2. RULE36-PILOT's byte comparisons and corpus_substitution.py run unchanged
#    on the result;
# 3. null_contexts.py confines the null view's differences to the dropped
#    rows' context digests;
# 4. RULE35-PILOT's strict token substitution runs on copies whose side
#    directory and build commit are respelled (respell_sides.py), since BASE
#    and TIP come from two measure runs.
# Usage: identity.sh WORKTREE BASE_SIDE TIP_SIDE OUT_DIR   (each side a
# `.../sides/<name>` directory; OUT_DIR must not exist)
set -u
BASE_REV=3e53c34fe100b72dd014224bfa87d8719d71b108
TIP_REV=051aef7f208120665b4d51e10dae6e2bb350fc31
W=$1
O=$4
cd "$W" || exit 2
mkdir "$O" || exit 2
mkdir "$O/sides" || exit 2
ln -s "$2" "$O/sides/base"
ln -s "$3" "$O/sides/tip"
M=$O/sides
R=docs/reports/rule36-pilot/scripts
S=docs/reports/arm-obligations-unit-grain/scripts
X=docs/reports/rule35-pilot/scripts/substitution_ids.py
python3 $S/narrowed_carriage.py "$M/base/corpus" "$M/tip/corpus" "$O/base-narrowed/corpus" \
  "$O/lists" >"$O/narrowed.txt"
echo $? >"$O/narrowed.exit"
N=$O/base-narrowed/corpus
T=$M/tip/corpus
python3 $R/compare_bytes.py "$M/base/system" "$M/tip/system" >"$O/bytes-system.txt"
python3 $R/compare_bytes.py "$M/base/slice" "$M/tip/slice" >"$O/bytes-slice.txt"
python3 $R/compare_bytes.py "$N/corpus" "$T/corpus" >"$O/bytes-corpus.txt"
python3 $R/corpus_substitution.py "$N/corpus" "$T/corpus" "$2" "$3" \
  >"$O/corpus-substitution.txt"
echo $? >"$O/corpus-substitution.exit"
python3 $S/null_contexts.py "$M/base/corpus/corpus-null.jsonl" "$T/corpus-null.jsonl" \
  "$O/lists/rows.tsv" >"$O/null-contexts.txt"
echo $? >"$O/null-contexts.exit"
P=$O/respelled
python3 $S/respell_sides.py "$N" "$2" "$BASE_REV" "$P/base/corpus" >/dev/null
python3 $S/respell_sides.py "$T" "$3" "$TIP_REV" "$P/tip/corpus" >/dev/null
C=$P/base/corpus/corpus
D=$P/tip/corpus/corpus
python3 $X \
  "$C/resource-diagnostics.json" "$D/resource-diagnostics.json" \
  "$C/resource-diagnostics.components.json" "$D/resource-diagnostics.components.json" \
  "$C/resource-diagnostics.contracts.json" "$D/resource-diagnostics.contracts.json" \
  "$C/resource-diagnostics.source-effects.json" "$D/resource-diagnostics.source-effects.json" \
  "$C/resource-diagnostics.chunk-contracts.json" "$D/resource-diagnostics.chunk-contracts.json" \
  "$P/base/corpus/corpus-null.summary.json" "$P/tip/corpus/corpus-null.summary.json" \
  "$P/base/corpus/corpus-outer.json" "$P/tip/corpus/corpus-outer.json" \
  >"$O/substitution-ids-corpus.jsonl"
echo $? >"$O/substitution-ids-corpus.exit"
for route in system slice; do
  python3 $S/respell_sides.py "$M/base/$route" "$2" "$BASE_REV" "$P/base/$route" >/dev/null
  python3 $S/respell_sides.py "$M/tip/$route" "$3" "$TIP_REV" "$P/tip/$route" >/dev/null
  B=$P/base/$route
  E=$P/tip/$route
  python3 $X \
    "$B/resource-diagnostics.json" "$E/resource-diagnostics.json" \
    "$B/resource-diagnostics.components.json" "$E/resource-diagnostics.components.json" \
    "$B/resource-diagnostics.contracts.json" "$E/resource-diagnostics.contracts.json" \
    "$B/resource-diagnostics.source-effects.json" "$E/resource-diagnostics.source-effects.json" \
    "$B/resource-diagnostics.chunk-contracts.json" "$E/resource-diagnostics.chunk-contracts.json" \
    "$B/$route-null.jsonl" "$E/$route-null.jsonl" \
    "$B/$route-null.summary.json" "$E/$route-null.summary.json" \
    "$B/$route-outer.json" "$E/$route-outer.json" \
    >"$O/substitution-ids-$route.jsonl"
  echo $? >"$O/substitution-ids-$route.exit"
done
