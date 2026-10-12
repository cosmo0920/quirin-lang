#!/bin/sh
# Campaign 1: execution_quota, base (diagnostic) binary versus the fixed binary,
# under the targeted loads that reproduced each mechanism.
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab7d2c81647aec5d7
E=$W/private/eqli
C=$W/crates/qcore-verifier-host
LB="python3 $E/scripts/loadbisect.py --cwd $C --warmup 1 --duration 1800"
$LB --binary $E/bin-diag/execution_quota --levels 0:0:0:0:0:0:20 --runs 30 --out $E/c1-base-child20.json --logs $E/c1-base-child20-logs
$LB --binary $E/bin-diag/execution_quota --levels 0:0:0:0:0:10 --runs 20 --out $E/c1-base-host10.json --logs $E/c1-base-host10-logs
$LB --binary $E/bin-after/execution_quota --levels 0:0:0:0:0:0:20 --runs 50 --out $E/c1-after-child20.json --logs $E/c1-after-child20-logs
$LB --binary $E/bin-after/execution_quota --levels 0:0:0:0:0:20 --runs 50 --out $E/c1-after-host20.json --logs $E/c1-after-host20-logs
$LB --binary $E/bin-after/execution_quota --levels 0:0:0:0:0:50 --runs 50 --out $E/c1-after-host50.json --logs $E/c1-after-host50-logs
echo DONE
