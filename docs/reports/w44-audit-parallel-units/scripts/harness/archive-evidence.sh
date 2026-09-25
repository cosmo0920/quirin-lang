#!/bin/sh
set -eu
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
report="$root/docs/reports/w44-audit-parallel-units"
cd "$root"
mkdir -p "$report/timing" "$report/identity" "$report/runs"
python3 "$report/scripts/timing_table.py" private/time \
  base=base-binary-j1 j1=branch-j1 j4=branch-j4 j8=branch-j8 \
  j16a=branch-j16-a j16b=branch-j16-b j16c=branch-j16-c j16d=branch-j16-d \
  census-j1=census-branch-j1 census-j16=census-branch-j16-forced-sequential \
  final-j16=final-binary-j16-confirmation final-j1=final-binary-j1-confirmation \
  > "$report/timing/pinned.json"
python3 "$report/scripts/timing_table.py" private/time \
  current-base=configured-base-binary-j1 current-j1=configured-branch-j1 current-j4=configured-branch-j4 \
  current-j8=configured-branch-j8 current-j16=configured-branch-j16 current-probe16=configured-branch-j16-probe \
  current-final-j16=configured-final-binary-j16-confirmation \
  > "$report/timing/configured.json"
for name in base j1 j4 j8 j16a j16b j16c j16d census-j1 census-j16 final-j16 final-j1; do
  cp "private/identity/$name.json" "$report/identity/baseline-identity-$name.json"
  cp "private/runs/$name/run.json" "$report/runs/run-$name.json"
done
for name in base-vs-j1 base-vs-j16a j1-vs-j4 j1-vs-j8 j1-vs-j16b j1-vs-j16c j1-vs-j16d j1-vs-final-j16 j1-vs-final-j1 census-j1-vs-census-j16 \
  current-base-vs-j1 current-base-vs-j4 current-base-vs-j8 current-base-vs-j16 current-base-vs-probe16 current-base-vs-final-j16; do
  cp "private/compare/$name.json" "$report/identity/compare-$name.json"
done
cp private/leaf/source-effects-j16d.json "$report/identity/source-effects-leaf-diff-vs-baseline-j16d.json"
cp private/logs/identity-all.log "$report/identity/identity-all.log"
cp private/logs/identity-final.log "$report/identity/identity-final.log"
for name in base j1 j4 j8 j16 probe16 final-j16; do
  sha256sum "private/runs-current/$name/audit.log" "private/runs-current/$name/resource-diagnostics.json" \
    "private/runs-current/$name/resource-diagnostics.contracts.json" | sed "s|$root/||"
  cat "private/runs-current/$name/binary.sha256" | sed "s|$root/||"
done > "$report/identity/configured-route-digests.txt"
echo archived
