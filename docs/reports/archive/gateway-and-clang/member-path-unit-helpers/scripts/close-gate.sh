#!/usr/bin/env bash
# close-gate.sh NAME -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: after
# gate-run.sh NAME, write gate.json from the logs (write_gate_json.py; it
# refuses any unaccounted nonzero exit) and finish the records
# (finish-gate.sh: matrix regeneration, Python suite, docs index). Exits to
# private/mpuh/gate-NAME/close-*.exit.
set -u
name="${1:-g1}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$repo/private/mpuh/gate-$name"
python3 "$here/write_gate_json.py" "$name" > "$logs/close-gate-json.log" 2>&1
echo $? > "$logs/close-gate-json.exit"
[ "$(cat "$logs/close-gate-json.exit")" = 0 ] || exit 1
bash "$here/finish-gate.sh" "$name"
