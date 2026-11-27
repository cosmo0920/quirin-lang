#!/bin/sh
# Campaign 2: the sibling binaries, base versus fixed, under the same targeted
# loads: host stall (c), the fork-window amplifier (b), and the account-wide
# task count plus CPU load for enforced_process_limits' RLIMIT_NPROC row (a).
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab7d2c81647aec5d7
E=$W/private/eqli
C=$W/crates/qcore-verifier-host
SHIM="--env LD_PRELOAD=$E/shim/amplify.so --env QCORE_AMPLIFY_CLOSE_US=20000"
LB="python3 $E/scripts/loadbisect.py --cwd $C --warmup 1 --duration 1800"
for b in confined_process_execution enforced_process_limits package_identity; do
  $LB --binary $E/bin-base/$b --levels 0:0:0:0:0:20,0:0:0:0:0:50 --runs 20 --out $E/c2-base-$b-stall.json --logs $E/c2-base-$b-stall-logs
  $LB --binary $E/bin-base/$b $SHIM --levels 0:0:0:0:0:0:50 --runs 20 --out $E/c2-base-$b-fork.json --logs $E/c2-base-$b-fork-logs
  $LB --binary $E/bin-after/$b --levels 0:0:0:0:0:20,0:0:0:0:0:50 --runs 50 --out $E/c2-after-$b-stall.json --logs $E/c2-after-$b-stall-logs
  $LB --binary $E/bin-after/$b $SHIM --levels 0:0:0:0:0:0:50 --runs 50 --out $E/c2-after-$b-fork.json --logs $E/c2-after-$b-fork-logs
done
$LB --binary $E/bin-base/enforced_process_limits --levels 3000:0:0,0:64:8 --runs 15 --out $E/c2-base-enforced-count.json --logs $E/c2-base-enforced-count-logs
$LB --binary $E/bin-after/enforced_process_limits --levels 3000:0:0,0:64:8 --runs 15 --out $E/c2-after-enforced-count.json --logs $E/c2-after-enforced-count-logs
echo DONE
