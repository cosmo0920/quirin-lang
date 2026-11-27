#!/usr/bin/env bash
# prepass.sh NAME BINARY -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: under the shared
# heavy lock, start the pinned corpus route with BINARY (copied to
# target/release/quirin-audit, the path run_pinned_audit.py runs) and
# QCORE_MEMBER_PREPASS_SECTION set, wait until the member-facts pre-pass has
# written its section to private/oprk/NAME-prepass.json, then stop that run
# (its own process group, started here with setsid; nothing else is
# signalled). A development aid for the pre-pass alone: the stopped run is no
# measurement. Exits and logs to private/oprk/NAME-prepass.*.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/oprk"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
flock "$lock" /usr/bin/env bash -c '
  repo="$1" out="$2" name="$3" binary="$4"
  mkdir -p "$repo/target/release"
  cat "$binary" > "$repo/target/release/quirin-audit" && chmod +x "$repo/target/release/quirin-audit"
  rm -f "$out/$name-prepass.json"
  start=$EPOCHREALTIME
  QCORE_MEMBER_PREPASS_SECTION="$out/$name-prepass.json" setsid python3 scripts/run_pinned_audit.py \
    "$out/$name-prepass-run" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$out/$name-prepass.log" 2>&1 &
  pid=$!
  while [ ! -s "$out/$name-prepass.json" ] && kill -0 "$pid" 2>/dev/null; do sleep 2; done
  sleep 1
  end=$EPOCHREALTIME
  kill -TERM -- "-$pid" 2>/dev/null
  wait "$pid" 2>/dev/null
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$out/$name-prepass.seconds"
' _ "$repo" "$out" "$name" "$binary"
test -s "$out/$name-prepass.json"; echo $? > "$out/$name-prepass.exit"
cat "$out/$name-prepass.exit" "$out/$name-prepass.seconds"
