#!/bin/sh
# census.sh RUN_DIR OUT_DIR : round 3's AST census over one run's report
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aab78fbbfefedf82b
S=$W/docs/reports/cg10-indirect-access-sole-residuals-r3/scripts
RUN=$1; OUT=$2
mkdir -p "$OUT"
export R3_CHECKOUT=$W/private/r4/fb-046accf6/fluent-bit-clang-source
python3 $S/prep_units.py "$RUN/resource-diagnostics.json" "$OUT/units" > "$OUT/prep.log" 2>&1 || { echo prep failed; exit 1; }
python3 $S/classify.py "$RUN/resource-diagnostics.json" "$OUT/units" "$OUT/classify" > "$OUT/classify.log" 2>&1 || { echo classify failed; exit 1; }
python3 $S/premise.py "$RUN/resource-diagnostics.json" "$OUT/classify/ind-leaves.jsonl.gz" "$OUT/premise" > "$OUT/premise.log" 2>&1 || { echo premise failed; exit 1; }
python3 $S/refine.py "$OUT/premise/premises.jsonl.gz" "$OUT/units" > "$OUT/refined-sole-shapes.md" 2> "$OUT/refine.log" || { echo refine failed; exit 1; }
echo ok
