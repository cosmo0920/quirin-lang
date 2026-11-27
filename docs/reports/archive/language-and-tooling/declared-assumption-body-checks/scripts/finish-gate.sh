#!/usr/bin/env bash
# finish-gate.sh NAME -- DECLARED-ASSUMPTION-BODY-CHECKS (adapted from
# VERDICT-BODY-STANDING-CLAUSE-SCOPE): after gate.sh into private/dabc/gate-NAME and
# write_gate_json.py NAME: copy the exits, heads and Clang version into gate/,
# regenerate the matrix with gate.json (09a), rerun the Python suite (09b), the
# docs index (10b) and its check (10c). No provider gate: providers/ is
# unchanged from the base.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/dabc/gate-$name"
report="$repo/docs/reports/declared-assumption-body-checks"
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
