#!/bin/sh
# The measurement series on the tree merged with main b22425ec: the whole run
# (archived as the new composition reference), checkpointed runs at 16 and 1
# jobs, compose, resume (no change, and the ChunkIO probe), the YAML/TOML round
# trips, the corpus arbiter against the W45-FB-BACKLOG-CUSTODY baseline, and
# the identity checks. One run at a time. usage: measure_merged.sh DIR COMMIT
M=$1; COMMIT=$2
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf || exit 2
S=docs/reports/w45-audit-checkpoints/scripts
R=docs/reports/w45-audit-checkpoints
REF=$R/artifacts/current-whole-merged
BI=docs/reports/w43-return-expression/scripts/baseline_identity.py
mkdir -p "$M" || exit 2
L=$M/series.log
step() { echo "== $1 $(date -u +%H:%M:%S)" >> $L; }
step whole-j16; python3 $S/run_current.py $M/whole-j16 --jobs 16 >> $L 2>&1
[ -d $REF ] || python3 docs/reports/w43-return-expression/scripts/archive.py $M/whole-j16 $REF > $M/archive.json 2>&1
step ck-j16; python3 $S/run_current.py $M/ck-j16 --jobs 16 --checkpoint-dir $M/qo-j16 >> $L 2>&1
step compose; python3 $S/run_current.py $M/compose --compose $M/qo-j16 >> $L 2>&1
step resume-j16; cp -r $M/qo-j16 $M/qo-resume; python3 $S/run_current.py $M/resume-j16 --jobs 16 --checkpoint-dir $M/qo-resume --resume >> $L 2>&1
step probe; python3 $S/probe_database.py $M/probe-db/compile_commands.json >> $L 2>&1
cp -r $M/qo-j16 $M/qo-probe
step probe-resume-j16; python3 $S/run_current.py $M/probe-resume-j16 --jobs 16 --compile-database $M/probe-db/compile_commands.json --checkpoint-dir $M/qo-probe --resume >> $L 2>&1
step probe-whole-j16; python3 $S/run_current.py $M/probe-whole-j16 --jobs 16 --compile-database $M/probe-db/compile_commands.json >> $L 2>&1
step roundtrip; sh $S/roundtrip.sh $M > $M/roundtrip.txt 2>&1; echo "roundtrip exit $?" >> $L
step arbiter; python3 docs/reports/w43-return-expression/scripts/run_audit.py $M/arbiter $COMMIT --jobs 16 > $M/arbiter.json 2>&1; echo "arbiter exit $?" >> $L
step ck-j1; python3 $S/run_current.py $M/ck-j1 --jobs 1 --checkpoint-dir $M/qo-j1 >> $L 2>&1
step whole-j1; python3 $S/run_current.py $M/whole-j1 --jobs 1 >> $L 2>&1
step identity
for run in ck-j16 compose resume-j16 ck-j1 whole-j1; do
  python3 $BI $M/$run $REF > $M/identity-$run.json 2>&1; echo "$run vs current-whole-merged exit $?" >> $L
done
python3 docs/reports/w43-return-expression/scripts/archive.py $M/probe-whole-j16 $M/probe-whole-archive > $M/probe-archive.json 2>&1
python3 $BI $M/probe-resume-j16 $M/probe-whole-archive > $M/identity-probe-resume-j16.json 2>&1; echo "probe-resume-j16 vs probe-whole exit $?" >> $L
python3 $BI $M/arbiter docs/reports/w45-fb-backlog-custody/integration/artifacts > $M/identity-arbiter.json 2>&1; echo "arbiter vs w45-fb-backlog-custody exit $?" >> $L
target/release/quirin-audit checkpoint verify $M/qo-j16/*.qo $M/qo-j1/*.qo $M/qo-resume/*.qo $M/qo-probe/*.qo > $M/verify.txt 2>&1; echo "verify exit $?" >> $L
step done
