#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
#
# Card GATE-SLOT-BUILDS-UNSLOTTED: run the slot tests that measure a spinner's
# CPU on one CPU shared with N other spinners, so the spinner gets about
# 1/(N+2) of a core (the 2026-11-14 failure was a share near 0.4).  Only one
# core is loaded, so other jobs on the host are not starved.  Every spinner is
# stopped by the pid captured when it was started.
#
#   starved_core.sh <scripts/tests directory> <log file> [cpu] [spinners]
set -u
TESTS=$1
LOG=$2
CPU=${3:-31}
N=${4:-15}
PIDS=""
i=0
while [ "$i" -lt "$N" ]; do
  taskset -c "$CPU" python3 -c 'while True: pass' &
  PIDS="$PIDS $!"
  i=$((i + 1))
done
sleep 1
cd "$TESTS" || exit 2
taskset -c "$CPU" python3 -m unittest -v \
  test_gate_slot_starvation.Drain.test_a_drain_gives_up_reopens_admission_releases_and_names_the_holder \
  test_gate_slot_starvation.Status.test_holders_a_draining_suite_and_a_waiter \
  test_gate_slot_starvation.Deadlines.test_a_tree_without_io_is_stopped_reported_and_only_its_own_processes_signalled \
  test_gate_slot_starvation.Deadlines.test_an_unreaped_zombie_does_not_hide_a_stall \
  >"$LOG" 2>&1
CODE=$?
for pid in $PIDS; do kill "$pid"; done
wait 2>/dev/null
echo "exit $CODE (cpu $CPU, $N competing spinners)" >>"$LOG"
exit "$CODE"
