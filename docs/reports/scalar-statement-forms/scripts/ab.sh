#!/bin/sh
# With/without rule 41 over the census units (no contracts). Usage: ab.sh WORKTREE
W=${1:?usage: ab.sh WORKTREE}
D=$W/target/release/deps/scalar_form_census-67db1e8398d77fde
QCORE_GATEWAY_RULES_OFF=41 python3 $W/docs/reports/scalar-statement-forms/scripts/run_census.py $D $W/private/census/pp $W/private/census/off 5 > $W/private/census/off-run.log 2>&1 &
python3 $W/docs/reports/scalar-statement-forms/scripts/run_census.py $D $W/private/census/pp $W/private/census/on 5 > $W/private/census/on-run.log 2>&1
wait
echo done >> $W/private/census/on-run.log
