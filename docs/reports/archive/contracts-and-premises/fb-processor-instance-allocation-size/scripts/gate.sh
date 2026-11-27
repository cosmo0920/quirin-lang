#!/usr/bin/env bash
# gate.sh -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: the records-only gate the
# brief names, after the ledger change: the acceptance matrix regenerated with
# the matrix's current gate provenance (no standing gate is run: no product
# code changed), the docs index and its check, then the Python suite (it reads
# the committed matrix) under the shared heavy lock, and the bold rule-id
# duplicate check. Logs and exits to private/fbpia/gate/; nothing piped into a
# gate step.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fbpia/gate"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/head"
python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/union-opaque-record-writes/gate.json > "$out/matrix.log" 2>&1
echo $? > "$out/matrix.exit"
python3 scripts/build_docs_index.py > "$out/index.log" 2>&1
echo $? > "$out/index.exit"
python3 scripts/build_docs_index.py --check > "$out/index-check.log" 2>&1
echo $? > "$out/index-check.exit"
flock "$lock" python3 -m unittest discover -s scripts/tests > "$out/python.log" 2>&1
echo $? > "$out/python.exit"
rg -o --no-filename '^[0-9]{1,2}[a-z]?\. \*\*' docs/agents/ > "$out/rule-ids.txt" 2>&1
sort "$out/rule-ids.txt" > "$out/rule-ids.sorted"
uniq -d "$out/rule-ids.sorted" > "$out/rule-ids.dup"
echo $? > "$out/rule-ids.exit"
git status --porcelain > "$out/status-after"
grep -H . "$out"/*.exit
wc -l < "$out/rule-ids.dup"
