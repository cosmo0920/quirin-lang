#!/usr/bin/env bash
# run.sh TREE VARIANT SCENARIO COUNT RUN_DIR -- run one harness scenario
# COUNT times, each in its own process under a watchdog
# (`timeout -k 5 60`: exit 124 means the watchdog fired, 137 that it had to
# kill), writing RUN_DIR/SCENARIO-NN.log, .exit and .cmd. The debug variant
# preloads joinlog.so; the tsan variant runs without the interposer (the
# sanitizer runtime in the executable intercepts pthread_* first) and keeps
# TSan's own report on stderr.
set -u
tree="$1"; variant="$2"; scenario="$3"; count="$4"; run="$5"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1
dir="$root/harness-$tree-$variant"
mkdir -p "$run" || exit 2
cd "$dir" || exit 2
ulimit -c 0   # no core files from the crashing scenario
for n in $(seq -w 1 "$count"); do
  base="$run/$scenario-$n"
  if [ "$variant" = tsan ]; then
    echo "TSAN_OPTIONS=halt_on_error=0:second_deadlock_stack=1 timeout -k 5 60 ./harness $scenario" > "$base.cmd"
    TSAN_OPTIONS=halt_on_error=0:second_deadlock_stack=1 timeout -k 5 60 ./harness "$scenario" > "$base.log" 2>&1
  else
    echo "LD_PRELOAD=./joinlog.so timeout -k 5 60 ./harness $scenario" > "$base.cmd"
    LD_PRELOAD=./joinlog.so timeout -k 5 60 ./harness "$scenario" > "$base.log" 2>&1
  fi
  echo $? > "$base.exit"
done
for f in "$run/$scenario"-*.exit; do printf '%s ' "$(cat "$f")"; done; echo
