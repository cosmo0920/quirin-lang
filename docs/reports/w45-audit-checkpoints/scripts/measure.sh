#!/bin/sh
# The final measurement series, one run at a time. usage: measure_final.sh M COMMIT
M=$1; COMMIT=$2
S=docs/reports/w45-audit-checkpoints/scripts
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf || exit 2
mkdir -p "$M" || exit 2
step() { echo "== $1 $(date -u +%H:%M:%S)" >> "$M/series.log"; }
step whole-j16; python3 $S/run_current.py "$M/whole-j16" --jobs 16 >> "$M/series.log" 2>&1
step ck-j16; python3 $S/run_current.py "$M/ck-j16" --jobs 16 --checkpoint-dir "$M/qo-j16" >> "$M/series.log" 2>&1
step compose; python3 $S/run_current.py "$M/compose" --compose "$M/qo-j16" >> "$M/series.log" 2>&1
step resume-j16; cp -r "$M/qo-j16" "$M/qo-resume"; python3 $S/run_current.py "$M/resume-j16" --jobs 16 --checkpoint-dir "$M/qo-resume" --resume >> "$M/series.log" 2>&1
step probe; python3 $S/probe_database.py "$M/probe-db/compile_commands.json" >> "$M/series.log" 2>&1
cp -r "$M/qo-j16" "$M/qo-probe"
step probe-resume-j16; python3 $S/run_current.py "$M/probe-resume-j16" --jobs 16 --compile-database "$M/probe-db/compile_commands.json" --checkpoint-dir "$M/qo-probe" --resume >> "$M/series.log" 2>&1
step probe-whole-j16; python3 $S/run_current.py "$M/probe-whole-j16" --jobs 16 --compile-database "$M/probe-db/compile_commands.json" >> "$M/series.log" 2>&1
step whole-j4; python3 $S/run_current.py "$M/whole-j4" --jobs 4 >> "$M/series.log" 2>&1
step ck-j4; python3 $S/run_current.py "$M/ck-j4" --jobs 4 --checkpoint-dir "$M/qo-j4" >> "$M/series.log" 2>&1
step whole-j1; python3 $S/run_current.py "$M/whole-j1" --jobs 1 >> "$M/series.log" 2>&1
step ck-j1; python3 $S/run_current.py "$M/ck-j1" --jobs 1 --checkpoint-dir "$M/qo-j1" >> "$M/series.log" 2>&1
step arbiter; python3 docs/reports/w43-return-expression/scripts/run_audit.py "$M/arbiter" "$COMMIT" --jobs 16 > "$M/arbiter.json" 2>&1; echo "arbiter exit $?" >> "$M/series.log"
step done
