#!/bin/sh
# Which amplifier makes the base confined binary fail with EPIPE?
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab7d2c81647aec5d7
E=$W/private/eqli
C=$W/crates/qcore-verifier-host
LB="python3 $E/scripts/loadbisect.py --cwd $C --warmup 0 --duration 600"
$LB --binary $E/bin-base/confined_process_execution --levels 0:0:0:0:0:0:50 --runs 15 --out $E/x-cpe-child.json --logs $E/x-cpe-child-logs > /dev/null 2>&1
$LB --binary $E/bin-base/confined_process_execution --levels 0:0:0 --runs 15 --env LD_PRELOAD=$E/shim/amplify.so --env QCORE_AMPLIFY_CLOSE_US=20000 --out $E/x-cpe-close.json --logs $E/x-cpe-close-logs > /dev/null 2>&1
python3 $E/scripts/summarize.py child=$E/x-cpe-child.json close=$E/x-cpe-close.json
