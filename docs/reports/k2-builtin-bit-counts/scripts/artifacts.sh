#!/bin/sh
# artifacts.sh -- write the report's compact measurement artifacts.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae71e0b9f85fdedfd
S=$W/docs/reports/k2-builtin-bit-counts/scripts
M=$W/private/k2bc/m
A=$W/docs/reports/k2-builtin-bit-counts/artifacts
mkdir -p "$A"
for n in base tip; do
  python3 "$S/totals.py" "$M/run-$n/resource-diagnostics.json" > "$A/current-totals-$n.json"
  python3 "$S/totals.py" "$M/sys-$n/resource-diagnostics.json" > "$A/system-totals-$n.json"
done
cp "$M/moved.jsonl" "$A/current-moved.jsonl"
cp "$M/changed.txt" "$A/current-changed.txt"
python3 "$S/moved.py" "$M/sys-base/resource-diagnostics.json" "$M/sys-tip/resource-diagnostics.json" > "$A/system-moved.jsonl"
python3 "$S/changed_rows.py" "$M/sys-base/resource-diagnostics.json" "$M/sys-tip/resource-diagnostics.json" > "$A/system-changed.txt"
cd "$M" && sha256sum run-base/resource-diagnostics.json run-tip/resource-diagnostics.json \
  sys-base/resource-diagnostics.json sys-tip/resource-diagnostics.json \
  sys-base/system-null.jsonl sys-tip/system-null.jsonl > "$A/raw-sha256.txt"
cat run-base/binary.sha256 run-tip/binary.sha256 | sed "s| $M/| |" > "$A/binaries.sha256"
python3 "$S/rows.py" "$M/run-base/resource-diagnostics.json" "$M/run-tip/resource-diagnostics.json" > "$A/current-bit-count-rows.json"
