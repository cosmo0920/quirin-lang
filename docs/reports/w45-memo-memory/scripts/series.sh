#!/bin/sh
# The measurement series of card W45-MEMO-MEMORY, one run at a time: the corpus
# arbiter at 16, 4 and 1 jobs and the checkpointed `quirin-audit current` at 16
# jobs, main and the change interleaved. usage: series.sh OUT_DIR BIN_MAIN BIN_CHANGE
# where BIN_* are directories holding cycle19_fluent_bit_libraries_audit and
# quirin-audit. Each step's exit goes to OUT_DIR/exits.txt.
M=$1; MAIN=$2; CHANGE=$3
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a8c17fc8aa8ae0abc || exit 2
S=docs/reports/w45-memo-memory/scripts/measure_variants.py
mkdir -p "$M" || exit 2
A1="main=$MAIN/cycle19_fluent_bit_libraries_audit"
A2="change=$CHANGE/cycle19_fluent_bit_libraries_audit"
Q1="main=$MAIN/quirin-audit"
Q2="change=$CHANGE/quirin-audit"
python3 $S "$M/arbiter-j16" 3 16 arbiter "$A1" "$A2" > "$M/arbiter-j16.out" 2>&1
echo "arbiter-j16 exit $?" >> "$M/exits.txt"
python3 $S "$M/ck-j16" 2 16 current-ck "$Q1" "$Q2" > "$M/ck-j16.out" 2>&1
echo "ck-j16 exit $?" >> "$M/exits.txt"
python3 $S "$M/arbiter-j4" 3 4 arbiter "$A1" "$A2" > "$M/arbiter-j4.out" 2>&1
echo "arbiter-j4 exit $?" >> "$M/exits.txt"
python3 $S "$M/arbiter-j1" 2 1 arbiter "$A1" "$A2" > "$M/arbiter-j1.out" 2>&1
echo "arbiter-j1 exit $?" >> "$M/exits.txt"
