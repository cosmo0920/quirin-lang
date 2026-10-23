#!/usr/bin/env bash
# W48-RK-LOG-CALLER-FRAME: BASE/TIP on the eight-home pinned-corpus route
# (the scheduler card's route: docs/reports/w48-sched-cleanup-production/
# scripts/measure.py with the batch4 link map), run under
#   flock /tmp/qcore-c-card-validation-20261022.lock pair8.sh RUN BASE_REV TIP_REV
# Each side is a `git archive` snapshot of its revision with the Libevent
# submodule cloned at its pinned commit, built in place (release
# quirin-audit), then both corpus routes run concurrently at --jobs 4.
# Every step's exit goes to a file; nothing is piped into a status.
set -u
wt=/media/Data/GitHub/quirin-lang/.claude/worktrees/w48-rk-log-caller-frame
run=$1
base_rev=$2
tip_rev=$3
mkdir "$run" || exit 2
date -u +%FT%TZ >"$run/lock-acquired"
free=$(df --output=avail -BG /media/Data | tail -1 | tr -dc 0-9)
echo "$free" >"$run/free-gib-start"
if [ "$free" -lt 40 ]; then echo "below 40 GiB: $free" >"$run/refused-disk"; exit 3; fi
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=6
declare -A rev=([base]=$(git -C "$wt" rev-parse "$base_rev") [tip]=$(git -C "$wt" rev-parse "$tip_rev"))
for side in base tip; do
  echo "${rev[$side]}" >"$run/$side.rev"
  snap="$run/snap-$side"
  mkdir -p "$snap"
  git -C "$wt" archive "${rev[$side]}" | tar -x -C "$snap" || { echo "archive $side" >"$run/refused-archive"; exit 4; }
  pin=$(git -C "$wt" ls-tree "${rev[$side]}" third_party/libevent | awk '{print $3}')
  echo "$pin" >"$run/$side.libevent"
  rmdir "$snap/third_party/libevent" 2>/dev/null || true
  git clone -q "$wt/third_party/libevent" "$snap/third_party/libevent" &&
    git -C "$snap/third_party/libevent" checkout -q "$pin" || { echo "libevent $side" >"$run/refused-libevent"; exit 4; }
done
for side in base tip; do
  (cd "$run/snap-$side" && cargo build --release -p qcore-audit --bin quirin-audit >"$run/build-$side.log" 2>&1
   echo $? >"$run/build-$side.exit") &
done
wait
for side in base tip; do
  if [ "$(cat "$run/build-$side.exit")" != 0 ]; then echo "build $side failed" >"$run/refused-build"; exit 5; fi
  sha256sum "$run/snap-$side/target/release/quirin-audit" >"$run/binary-$side.sha256"
done
date -u +%FT%TZ >"$run/built"
# The scheduler card's driver, byte for byte: the frozen pinned inputs'
# compile database spells the primary checkout through its symlink
# (/home/cosmo/Documents/GitHub/qcore-c), so a driver rooted at the real
# directory selects no source (attempt pair8-1: "compile database contains no
# selected cmetrics sources", exit 1 on both sides). Inputs are only read.
measure="$wt/docs/reports/w48-sched-cleanup-production/scripts/measure.py"
map="$wt/docs/reports/w48-sched-cleanup-production/link-map.corpus.json"
for side in base tip; do
  (python3 "$measure" corpus "$run/$side-corpus" --link-map "$map" --inputs-snapshot "$run/snap-$side" \
     --binary-source "${rev[$side]}" --jobs 4 >"$run/$side-corpus.driver.log" 2>&1
   echo $? >"$run/$side-corpus.exit") &
done
wait
df --output=avail -BG /media/Data | tail -1 >"$run/free-gib-end"
date -u +%FT%TZ >"$run/done"
