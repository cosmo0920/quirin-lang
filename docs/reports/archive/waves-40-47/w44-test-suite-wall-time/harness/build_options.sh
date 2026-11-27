#!/bin/bash
# Usage (from the worktree root): build_options.sh <outdir> <label> [--config ...]...
# From a clean target/, times `cargo test --workspace --no-run` under the given
# profile override, records peak RSS, uptime and target/ size, then cleans
# target/ again so at most one profile build is on disk at a time.
out="$1/$2"
shift 2
mkdir -p "$out"
export CARGO_INCREMENTAL=0
cargo clean >"$out/clean.log" 2>&1
df -h /home >"$out/df.before"
uptime >"$out/uptime.before"
/usr/bin/time -v -o "$out/time.txt" cargo test --workspace --no-run "$@" >"$out/build.log" 2>&1
echo $? >"$out/exit"
uptime >"$out/uptime.after"
du -sb target >"$out/target.bytes"
du -sh target >"$out/target.human"
cargo clean >>"$out/clean.log" 2>&1
echo "$(cat "$out/exit") $(grep Elapsed "$out/time.txt" | awk '{print $NF}') $(cat "$out/target.human" | awk '{print $1}') load $(awk -F'load average: ' '{print $2}' "$out/uptime.before")" | tee "$out/report.txt"
