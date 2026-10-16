#!/usr/bin/env bash
# finish-gate.sh NAME PROVIDER -- OUTER-CONTRACT (from FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE): after gate.sh into private/oc/gate-NAME,
# provider-gate.sh into private/oc/provider-gate-PROVIDER
# and gate.json written: copy the exits, heads and Clang version into gate/,
# regenerate the matrix with gate.json (09a), rerun the Python suite (09b), the
# docs index (10b) and its check (10c).
set -u
name="$1"
provider="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/oc/gate-$name"
report="$repo/docs/reports/outer-contract"
cd "$repo" || exit 2
mkdir -p "$report/gate" || exit 2
cp "$logs"/*.exit "$logs/head" "$logs/docs-head" "$report/gate/"
cp "$logs/clang-version.txt" "$report/gate/clang-version.txt"
for f in "$repo/private/oc/provider-gate-$provider"/*.exit; do cp "$f" "$report/gate/provider-$(basename "$f")"; done
cp "$repo/private/oc/provider-gate-$provider/head" "$report/gate/provider-head"
python3 docs/reports/remaining-d02/build_matrix.py . --gate "$report/gate.json" > "$logs/09a-matrix.log" 2>&1
echo $? > "$report/gate/09a-matrix.exit"
python3 -m unittest discover -s scripts/tests > "$logs/09b-python.log" 2>&1
echo $? > "$report/gate/09b-python.exit"
python3 scripts/build_docs_index.py > "$logs/10b-index.log" 2>&1
echo $? > "$report/gate/10b-index.exit"
python3 scripts/build_docs_index.py --check > "$logs/10c-index-check.log" 2>&1
echo $? > "$report/gate/10c-index-check.exit"
grep -H . "$report"/gate/*.exit
