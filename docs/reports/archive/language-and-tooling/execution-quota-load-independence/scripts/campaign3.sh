#!/bin/sh
# Campaign 3: window amplifiers (LD_PRELOAD of private/eqli/shim/amplify.so into
# the measured binary only), base (diagnostic) binary versus the fixed binary.
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab7d2c81647aec5d7
E=$W/private/eqli
C=$W/crates/qcore-verifier-host
SHIM="--env LD_PRELOAD=$E/shim/amplify.so"
LB="python3 $E/scripts/loadbisect.py --cwd $C --warmup 0 --duration 1800"
# (b) fork window: close delay on provider.sh write descriptors, forked-child stall.
$LB --binary $E/bin-diag/execution_quota $SHIM --env QCORE_AMPLIFY_CLOSE_US=20000 --levels 0:0:0:0:0:0:0 --runs 20 --out $E/c3-base-close20.json --logs $E/c3-base-close20-logs
$LB --binary $E/bin-diag/execution_quota $SHIM --env QCORE_AMPLIFY_CLOSE_US=20000 --levels 0:0:0:0:0:0:5,0:0:0:0:0:0:20,0:0:0:0:0:0:50 --runs 20 --out $E/c3-base-close20-child.json --logs $E/c3-base-close20-child-logs
$LB --binary $E/bin-after/execution_quota $SHIM --env QCORE_AMPLIFY_CLOSE_US=20000 --levels 0:0:0:0:0:0:50 --runs 50 --out $E/c3-after-close20-child50.json --logs $E/c3-after-close20-child50-logs
# (d) coarse wall clock.
$LB --binary $E/bin-diag/execution_quota $SHIM --env QCORE_AMPLIFY_REALTIME_NS=1000 --levels 0:0:0 --runs 20 --out $E/c3-base-clock1us.json --logs $E/c3-base-clock1us-logs
$LB --binary $E/bin-diag/execution_quota $SHIM --env QCORE_AMPLIFY_REALTIME_NS=1000000 --levels 0:0:0 --runs 20 --out $E/c3-base-clock1ms.json --logs $E/c3-base-clock1ms-logs
$LB --binary $E/bin-after/execution_quota $SHIM --env QCORE_AMPLIFY_REALTIME_NS=1000000 --levels 0:0:0 --runs 50 --out $E/c3-after-clock1ms.json --logs $E/c3-after-clock1ms-logs
echo DONE
