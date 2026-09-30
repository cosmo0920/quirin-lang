#!/bin/sh
# CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: the gate steps after the matrix
# regeneration. usage: gate_after.sh LOG_DIR (relative to the checkout root).
# Every exit is captured to LOG_DIR/<step>.exit; no step is piped.
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
mkdir -p "$logs"
step() {
  name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
}
step 09a-matrix python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/cg07-generic-and-statement-expressions/gate.json
step 09b-python python3 -m unittest discover -s scripts/tests
step 10b-index python3 scripts/build_docs_index.py
