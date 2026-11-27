#!/bin/sh
# dbg.sh -- build private/m/side-dbg (tip plus debug prints) and run the
# system route with QDBG set to the enqueue.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a886acca29e222359
T=$W/private/m/side-dbg
mkdir -p $W/private/m/sys-dbg
cd $T || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > $W/private/m/sys-dbg/build.log 2>&1 || exit 3
export QDBG=flb_output_task_queue_enqueue
sh $W/docs/reports/leri-held-exit-class-precondition/scripts/system.sh "$T" $W/private/m/sys-dbg
