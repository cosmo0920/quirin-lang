#!/usr/bin/env bash
# artifacts.sh BEFORE AFTER -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: from the two
# measured runs under private/lpp/ (and the slice runs under the shared
# prepared attempt), write the report's artifacts: the summary of both runs,
# the rows resting on the 34 with the comparison, and each route's report
# differences by kind. Exits to private/lpp/artifacts.*.exit.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
art="$repo/docs/reports/librdkafka-plugin-premise/artifacts"
priv="$repo/private/lpp"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
python3 "$here/summary.py" "$before" "$after" > "$art/summary.json"
echo $? > "$priv/artifacts.summary.exit"
python3 "$here/premise_rows.py" "$before" "$after" "$priv/rows-$after.json" > "$priv/rows-$after.out"
echo $? > "$priv/artifacts.rows.exit"
python3 "$here/rows_table.py" "$priv/rows-$after.json" "$after" "$art/rows-after.json" > "$priv/safe-rows-after.md"
echo $? > "$priv/artifacts.table.exit"
python3 "$here/report_diff.py" "$priv/$before/resource-diagnostics.json" "$priv/$after/resource-diagnostics.json" > "$art/report-diff-corpus.txt"
echo $? > "$priv/artifacts.diff-corpus.exit"
python3 "$here/report_diff.py" "$priv/$before-system/resource-diagnostics.json" "$priv/$after-system/resource-diagnostics.json" > "$art/report-diff-system.txt"
echo $? > "$priv/artifacts.diff-system.exit"
python3 "$here/report_diff.py" "$slice/lpp-$before/resource-diagnostics.json" "$slice/lpp-$after/resource-diagnostics.json" > "$art/report-diff-slice.txt"
echo $? > "$priv/artifacts.diff-slice.exit"
grep -H . "$priv"/artifacts.*.exit
