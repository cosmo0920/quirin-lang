#!/usr/bin/env bash
# finish-gate.sh NAME -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (from DEEP-AST-TRANSPORTS-AND-WALL-TIME) (adapted from RDHTTP-HEADER-SUMMARY-GAP, itself adapted from DECLARED-ASSUMPTION-BODY-CHECKS,
# VERDICT-BODY-STANDING-CLAUSE-SCOPE): after gate.sh into private/cg10r2/gate-NAME and
# write_gate_json.py NAME: copy the exits, heads and Clang version into gate/,
# regenerate the matrix with gate.json (09a), rerun the Python suite (09b), the
# docs index (10b) and its check (10c). providers/ changes, so the provider
# gate (provider-gate.sh) runs on its own.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/cg10r2/gate-$name"
report="$repo/docs/reports/cg10-indirect-access-sole-residuals-r2"
cd "$repo" || exit 2
mkdir -p "$report/gate" || exit 2
cp "$logs"/*.exit "$logs/head" "$logs/docs-head" "$report/gate/"
cp "$logs/clang-version.txt" "$report/gate/clang-version.txt"
python3 docs/reports/remaining-d02/build_matrix.py . --gate "$report/gate.json" > "$logs/09a-matrix.log" 2>&1
echo $? > "$report/gate/09a-matrix.exit"
python3 -m unittest discover -s scripts/tests > "$logs/09b-python.log" 2>&1
echo $? > "$report/gate/09b-python.exit"
python3 scripts/build_docs_index.py > "$logs/10b-index.log" 2>&1
echo $? > "$report/gate/10b-index.exit"
python3 scripts/build_docs_index.py --check > "$logs/10c-index-check.log" 2>&1
echo $? > "$report/gate/10c-index-check.exit"
grep -H . "$report"/gate/*.exit
