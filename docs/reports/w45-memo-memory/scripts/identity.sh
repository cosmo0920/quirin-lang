#!/bin/sh
# Baseline identity of the first main and change arbiter run at each job count
# against W45-FB-BACKLOG-CUSTODY's integration baseline, and the leaf classes of
# the checkpointed run against W45-AUDIT-CHECKPOINTS's merged whole-run
# reference. usage: identity.sh SERIES_DIR EVIDENCE_DIR
M=$1; E=$2
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a8c17fc8aa8ae0abc
cd $W || exit 2
mkdir -p "$E" || exit 2
BI=docs/reports/w43-return-expression/scripts/baseline_identity.py
BASE=docs/reports/w45-fb-backlog-custody/integration/artifacts
for s in arbiter-j16 arbiter-j4 arbiter-j1; do
  for v in main-1 change-1; do
    python3 $BI "$M/$s/$v" $BASE --root-alias "$W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" > "$E/baseline-identity-$s-$v.json" 2>&1
    echo "$s $v exit $?" >> "$E/identity-exits.txt"
  done
done
REF=docs/reports/w45-audit-checkpoints/artifacts/current-whole-merged
for v in main-1 change-1; do
  python3 docs/reports/w45-memo-memory/scripts/leaf_classes.py "$M/ck-j16/$v/resource-diagnostics.json" $REF/diagnostics.json.gz "$W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf" > "$E/ck-j16-$v-leaf-classes.json" 2>&1
  echo "ck-j16 $v leaf classes exit $?" >> "$E/identity-exits.txt"
done
