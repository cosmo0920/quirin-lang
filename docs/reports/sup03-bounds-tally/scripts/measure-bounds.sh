#!/usr/bin/env bash
# measure-bounds.sh NAME -- SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN (adapted from
# CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS): the pinned corpus arbiter
# (66910c10) with the bounds property selected (QCORE_BOUNDS_PROPERTY), using
# the release binary build.sh built in this checkout, under the shared heavy
# lock, wall time measured inside the lock. Output:
# private/sup03/NAME-bounds/ (the arbiter's directory),
# private/sup03/NAME-bounds.jsonl (bounds rows), .summary.json (the run's own
# tally), .contracts.json (the installed bounds environment), .log/.stderr/
# .exit/.seconds. Exits to files; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sup03"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/$name-bounds.head"
sha256sum target/release/quirin-audit > "$out/$name-bounds.binary.sha256" || exit 3
flock "$lock" /usr/bin/env bash -c '
  start=$EPOCHREALTIME
  QCORE_BOUNDS_PROPERTY="$1-bounds.jsonl" python3 scripts/run_pinned_audit.py "$1-bounds" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$1-bounds.log" 2> "$1-bounds.stderr"
  rc=$?
  end=$EPOCHREALTIME
  echo "$rc" > "$1-bounds.exit"
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$1-bounds.seconds"
' _ "$out/$name"
cat "$out/$name-bounds.exit"
