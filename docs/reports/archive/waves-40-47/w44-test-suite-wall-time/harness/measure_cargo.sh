#!/bin/bash
# Usage (from the worktree root):
#   measure_cargo.sh <outdir> <command...>
# Cleans the dev profile, then times one command with /usr/bin/time -v while
# timestamping its merged output. Records uptime before/after, the command's
# own exit status (not a pipe's), and the size of target/ afterwards.
# The command is run as given, e.g. `cargo test --workspace --no-fail-fast`
# or `python3 scripts/run_test_binaries.py`.
out="$1"
shift
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$out"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
cargo clean --profile dev >"$out/clean.log" 2>&1
df -h /home >"$out/df.before"
uptime >"$out/uptime.before"
date -Is >"$out/start"
/usr/bin/time -v -o "$out/time.txt" bash -c \
  '"$@" 2>&1 | python3 "'"$here"'/ts.py" >"'"$out"'/output.log"; exit ${PIPESTATUS[0]}' _ "$@"
echo $? >"$out/exit"
date -Is >"$out/end"
uptime >"$out/uptime.after"
du -sb target >"$out/target.bytes" 2>&1
du -sh target >"$out/target.human" 2>&1
