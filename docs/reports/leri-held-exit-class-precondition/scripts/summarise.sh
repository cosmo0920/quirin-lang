#!/bin/sh
# summarise.sh -- the compact artifacts of private/m/{sys,census}-{base,tip}:
# totals, moved rows, changed rows and the SHA-256 digests of the raw reports
# and binaries (which stay in private/).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-abd604af97d27e262
R=$W/docs/reports/leri-held-exit-class-precondition
S=$R/scripts
A=$R/artifacts
M=$W/private/m
mkdir -p "$A"
for route in sys census; do
  for side in base tip; do
    python3 "$S/totals.py" "$M/$route-$side/resource-diagnostics.json" > "$A/$route-totals-$side.json"
  done
  python3 "$S/moved.py" "$M/$route-base/resource-diagnostics.json" "$M/$route-tip/resource-diagnostics.json" > "$A/$route-moved.jsonl"
  python3 "$S/changed_rows.py" "$M/$route-base/resource-diagnostics.json" "$M/$route-tip/resource-diagnostics.json" > "$A/$route-changed.txt"
done
cd "$W" || exit 2
sha256sum private/m/sys-base/resource-diagnostics.json private/m/sys-tip/resource-diagnostics.json \
  private/m/census-base/resource-diagnostics.json private/m/census-tip/resource-diagnostics.json \
  private/m/sys-base/system-null.jsonl private/m/sys-tip/system-null.jsonl > "$A/digests.txt"
sed "s|$W/||" private/m/sys-base/binary.sha256 private/m/sys-tip/binary.sha256 >> "$A/digests.txt"
