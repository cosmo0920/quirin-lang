#!/bin/sh
# The streamed-setup memory series (card W45-AUDIT-CHECKPOINTS, item 4 of the
# integration review): one run at a time, then the identity checks.
# usage: measure_memory.sh MEASURE_DIR COMMIT
M=$1; COMMIT=$2
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf || exit 2
S=docs/reports/w45-audit-checkpoints/scripts
R=docs/reports/w45-audit-checkpoints
BI=docs/reports/w43-return-expression/scripts/baseline_identity.py
mkdir -p "$M" || exit 2
L=$M/series.log
step() { echo "== $1 $(date -u +%H:%M:%S)" >> $L; }
step ck-j1; python3 $S/run_current.py $M/ck-j1 --jobs 1 --checkpoint-dir $M/qo-j1 >> $L 2>&1
step whole-j1; python3 $S/run_current.py $M/whole-j1 --jobs 1 >> $L 2>&1
step resume-j1; cp -r $M/qo-j1 $M/qo-resume-j1; python3 $S/run_current.py $M/resume-j1 --jobs 1 --checkpoint-dir $M/qo-resume-j1 --resume >> $L 2>&1
step ck-j4; python3 $S/run_current.py $M/ck-j4 --jobs 4 --checkpoint-dir $M/qo-j4 >> $L 2>&1
step whole-j4; python3 $S/run_current.py $M/whole-j4 --jobs 4 >> $L 2>&1
step ck-j16; python3 $S/run_current.py $M/ck-j16 --jobs 16 --checkpoint-dir $M/qo-j16 >> $L 2>&1
step whole-j16; python3 $S/run_current.py $M/whole-j16 --jobs 16 >> $L 2>&1
step compose; python3 $S/run_current.py $M/compose --compose $M/qo-j16 >> $L 2>&1
step arbiter; python3 docs/reports/w43-return-expression/scripts/run_audit.py $M/arbiter $COMMIT --jobs 16 > $M/arbiter.json 2>&1; echo "arbiter exit $?" >> $L
step identity
for run in ck-j1 whole-j1 resume-j1 ck-j4 whole-j4 ck-j16 whole-j16 compose; do
  python3 $BI $M/$run $R/artifacts/current-whole > $M/identity-$run.json 2>&1; echo "$run vs current-whole exit $?" >> $L
done
python3 $BI $M/arbiter docs/reports/w45-fb-retry-lifetime/integration/artifacts > $M/identity-arbiter.json 2>&1; echo "arbiter vs w45-fb-retry-lifetime exit $?" >> $L
for f in resource-diagnostics.contracts.json resource-diagnostics.source-effects.json; do
  cmp -s $M/arbiter/$f private/measure-final/arbiter/$f; echo "arbiter $f vs pre-streaming arbiter cmp $?" >> $L
done
target/release/quirin-audit checkpoint verify $M/qo-j1/*.qo $M/qo-j4/*.qo $M/qo-j16/*.qo > $M/verify.txt 2>&1; echo "verify exit $?" >> $L
step done
