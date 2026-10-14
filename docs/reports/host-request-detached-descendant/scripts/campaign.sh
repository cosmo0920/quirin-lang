#!/bin/sh
# campaign.sh -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT (from
# HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's): the probe binary
# (tests/request_consumption.rs: the earlier card's nine cases plus this
# card's nine, whose request pipe a background descendant keeps after the
# direct child's exit with both outputs closed) at the base and at this tip,
# under each condition that moves the provider's exit, the descendant's reads
# and the host's decision relative to one another:
#   idle;
#   host stall 20 and 50 ms (the test process alone SIGSTOPped, its children
#     keep running);
#   fifo_delay.so first request write delayed 30 ms;
#   fifo_delay.so close of each pipe write end delayed 30 ms (also delays
#     std's spawn by about 90 ms, and each holder check's write end);
#   both delays together with a 20 ms host stall.
# Base 20 runs per condition (the base binary's assertions are the
# base-compatible "not Completed, exit 0" / "Completed"); tip 50 runs per
# condition. Each side runs under the shared heavy lock, so no arbiter or
# build of this card runs beside it. Every load is bounded by loadbisect.py's
# --duration and killed by PID.
R=$(cd "$(dirname "$0")/../../../.." && pwd)
H=$R/private/hdd
C=$R/crates/qcore-verifier-host
S=$H/shim/fifo_delay.so
L=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
LB="python3 $R/docs/reports/host-request-detached-descendant/scripts/loadbisect.py --cwd $C --warmup 1 --duration 3600"
W="--env LD_PRELOAD=$S --env QCORE_DELAY_FIFO_WRITE_US=30000"
K="--env LD_PRELOAD=$S --env QCORE_DELAY_FIFO_CLOSE_US=30000"
B="--env LD_PRELOAD=$S --env QCORE_DELAY_FIFO_WRITE_US=30000 --env QCORE_DELAY_FIFO_CLOSE_US=30000"
mkdir -p "$H/campaign" || exit 2
for side in ${SIDES:-base after}; do
  case "$side" in base*) runs=${RUNS:-20};; *) runs=${RUNS:-50};; esac
  bin=$H/bin-$side/request_consumption
  out=$H/campaign/$side
  flock "$L" sh -c "
    $LB --binary $bin --levels 0:0:0,0:0:0:0:0:20,0:0:0:0:0:50 --runs $runs --out $out-stall.json --logs $out-stall-logs
    $LB --binary $bin $W --levels 0:0:0 --runs $runs --out $out-write.json --logs $out-write-logs
    $LB --binary $bin $K --levels 0:0:0 --runs $runs --out $out-close.json --logs $out-close-logs
    $LB --binary $bin $B --levels 0:0:0:0:0:20 --runs $runs --out $out-both.json --logs $out-both-logs
  "
  echo "$?" > "$H/campaign/$side.exit"
done
echo DONE
