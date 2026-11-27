#!/bin/sh
# Identity comparisons over the final measurement series. usage: identity_all.sh M
M=$1
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf || exit 2
R=docs/reports/w45-audit-checkpoints
BI=docs/reports/w43-return-expression/scripts/baseline_identity.py
ALIAS=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
O=$M/identity
mkdir -p $O
[ -d $R/artifacts/current-whole ] || python3 docs/reports/w43-return-expression/scripts/archive.py $M/whole-j16 $R/artifacts/current-whole > $O/archive.json 2>&1
for run in whole-j4 whole-j1 ck-j16 ck-j4 ck-j1 compose resume-j16; do
  python3 $BI $M/$run $R/artifacts/current-whole > $O/$run.json 2>&1; echo "$run exit $?" >> $O/summary.txt
done
python3 docs/reports/w43-return-expression/scripts/archive.py $M/probe-whole-j16 $M/probe-whole-archive > $O/probe-archive.json 2>&1
python3 $BI $M/probe-resume-j16 $M/probe-whole-archive > $O/probe-resume-j16.json 2>&1; echo "probe-resume-j16 vs probe-whole exit $?" >> $O/summary.txt
python3 $BI $M/arbiter docs/reports/w45-fb-retry-lifetime/integration/artifacts --root-alias $ALIAS > $O/arbiter.json 2>&1; echo "arbiter vs w45-fb-retry-lifetime exit $?" >> $O/summary.txt
for dir in qo-j16 qo-j4 qo-j1 qo-resume qo-probe; do
  target/release/quirin-audit checkpoint verify $M/$dir/*.qo > $O/verify-$dir.txt 2>&1; echo "verify $dir exit $?" >> $O/summary.txt
done
cat $O/summary.txt
