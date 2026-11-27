#!/usr/bin/env bash
# finish-gate.sh NAME -- LEAN-4-35-TOOLCHAIN (from OP-CALLBACK-MEMBER-PATH-RELEASE's):
# after gate-whole.sh into private/lean-4-35-toolchain/gate-NAME and gate.json
# written: copy the exits, head, Lean pin and version and the Clang version
# into gate/, regenerate the matrix with gate.json (09a), rerun the Python
# suite (09b), the docs index (10b) and its check (10c).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/lean-4-35-toolchain/gate-$name"
report="$repo/docs/reports/lean-4-35-toolchain"
cd "$repo" || exit 2
mkdir -p "$report/gate" || exit 2
cp "$logs"/*.exit "$logs/head" "$logs/lean-toolchain.txt" "$logs/lean-version.txt" "$report/gate/"
cp "$logs/clang-version.txt" "$report/gate/clang-version.txt"
# A repository-relative --gate path: the matrix records it as its provenance.
python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/lean-4-35-toolchain/gate.json > "$logs/09a-matrix.log" 2>&1
echo $? > "$report/gate/09a-matrix.exit"
python3 -m unittest discover -s scripts/tests > "$logs/09b-python.log" 2>&1
echo $? > "$report/gate/09b-python.exit"
python3 scripts/build_docs_index.py > "$logs/10b-index.log" 2>&1
echo $? > "$report/gate/10b-index.exit"
python3 scripts/build_docs_index.py --check > "$logs/10c-index-check.log" 2>&1
echo $? > "$report/gate/10c-index-check.exit"
grep -H . "$report"/gate/*.exit
