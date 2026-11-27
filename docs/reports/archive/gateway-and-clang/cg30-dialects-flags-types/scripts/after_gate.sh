#!/bin/sh
# CG30-DIALECTS-FLAGS-TYPES (from CG25-RESOURCE-KINDS' final_gate.sh, its
# `matrix` step): after the gate, archive the exits, regenerate the matrix's
# derived values from gate.json, then rerun the steps that read them (light; no
# lock needed). usage: after_gate.sh LOG_DIR (relative to the checkout root).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
report="$root/docs/reports/cg30-dialects-flags-types"
mkdir -p "$report/gate"
cp "$logs"/*.exit "$logs/clang-version.txt" "$report/gate/"
python3 docs/reports/remaining-d02/build_matrix.py . --gate "$report/gate.json" \
  > "$logs/09a-matrix.log" 2>&1
echo $? > "$report/gate/09a-matrix.exit"
python3 -m unittest discover -s scripts/tests > "$logs/09b-python.log" 2>&1
echo $? > "$report/gate/09b-python.exit"
python3 scripts/build_docs_index.py > "$logs/10b-index.log" 2>&1
echo $? > "$report/gate/10b-index.exit"
python3 scripts/build_docs_index.py --check > "$logs/10c-index-check.log" 2>&1
echo $? > "$report/gate/10c-index-check.exit"
