#!/bin/sh
# campaign.sh -- HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: the probe binary
# (tests/request_consumption.rs: five non-consuming and four consuming
# providers, frames under and over the pipe capacity) at the base and at this
# tip, under each condition that moves the provider's exit relative to the
# host's write:
#   idle;
#   host stall 20 and 50 ms (the test process alone SIGSTOPped, its children
#     keep running: the exit comes first);
#   fifo_delay.so first request write delayed 30 ms (the exit comes first);
#   fifo_delay.so request close delayed 30 ms (an exact reader exits before
#     the host closes its end);
#   both delays together with a 20 ms host stall.
# Base 20 runs per condition (the base binary's assertions are the
# base-compatible "not Completed"/"Completed"); tip 50 runs per condition.
# Every load is bounded by loadbisect.py's --duration and killed by PID.
R=$(cd "$(dirname "$0")/../../../.." && pwd)
H=$R/private/hur
C=$R/crates/qcore-verifier-host
S=$H/shim/fifo_delay.so
LB="python3 $R/docs/reports/host-unread-request-acceptance/scripts/loadbisect.py --cwd $C --warmup 1 --duration 1800"
W="--env LD_PRELOAD=$S --env QCORE_DELAY_FIFO_WRITE_US=30000"
K="--env LD_PRELOAD=$S --env QCORE_DELAY_FIFO_CLOSE_US=30000"
B="--env LD_PRELOAD=$S --env QCORE_DELAY_FIFO_WRITE_US=30000 --env QCORE_DELAY_FIFO_CLOSE_US=30000"
mkdir -p "$H/campaign" || exit 2
for side in ${SIDES:-base after}; do
  case "$side" in base*) runs=20;; *) runs=50;; esac
  bin=$H/bin-$side/request_consumption
  out=$H/campaign/$side
  $LB --binary "$bin" --levels 0:0:0,0:0:0:0:0:20,0:0:0:0:0:50 --runs $runs --out "$out-stall.json" --logs "$out-stall-logs"
  $LB --binary "$bin" $W --levels 0:0:0 --runs $runs --out "$out-write.json" --logs "$out-write-logs"
  $LB --binary "$bin" $K --levels 0:0:0 --runs $runs --out "$out-close.json" --logs "$out-close-logs"
  $LB --binary "$bin" $B --levels 0:0:0:0:0:20 --runs $runs --out "$out-both.json" --logs "$out-both-logs"
done
echo DONE
