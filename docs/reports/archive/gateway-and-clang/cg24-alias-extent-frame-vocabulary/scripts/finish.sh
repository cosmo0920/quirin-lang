#!/usr/bin/env bash
# finish.sh -- CG24-ALIAS-EXTENT-FRAME-VOCABULARY's gate records (from CG29's):
# copy the gate exits and the Clang recording beside gate.json, regenerate the
# matrix with this gate (09a), rerun the Python suite (09b), regenerate the docs
# index and check it (10a, 10b). Each exit is written to private/cg24/gate;
# nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
r="$here/.."
g="$repo/private/cg24/gate"
cd "$repo" || exit 2
mkdir -p "$r/gate"
cp "$g"/*.exit "$g/clang-version.txt" "$r/gate/"
python3 docs/reports/remaining-d02/build_matrix.py . --gate "$r/gate.json" > "$g/09a.log" 2>&1
echo $? > "$g/09a-matrix.exit"
python3 -m unittest discover -s scripts/tests > "$g/09b.log" 2>&1
echo $? > "$g/09b-python.exit"
python3 scripts/build_docs_index.py > "$g/10a.log" 2>&1
echo $? > "$g/10a-index.exit"
python3 scripts/build_docs_index.py --check > "$g/10b.log" 2>&1
echo $? > "$g/10b-index.exit"
cp "$g"/09a-matrix.exit "$g"/09b-python.exit "$g"/10a-index.exit "$g"/10b-index.exit "$r/gate/"
