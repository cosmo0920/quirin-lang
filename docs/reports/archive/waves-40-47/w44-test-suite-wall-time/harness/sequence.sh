#!/bin/bash
# Usage (from the worktree root): sequence.sh <run>...
# Each <run> is `plain:<outdir>` (plain `cargo test --workspace --no-fail-fast`)
# or `runner:<outdir>[:<jobs>]` (scripts/run_test_binaries.py). Every run starts
# from `cargo clean --profile dev` (measure_cargo.sh) and target/ is cleaned
# fully afterwards, so at most one build is on disk at a time.
here="$(cd "$(dirname "$0")" && pwd)"
for run in "$@"; do
  kind="${run%%:*}"
  rest="${run#*:}"
  out="${rest%%:*}"
  jobs="${rest#*:}"
  [ "$jobs" = "$rest" ] && jobs=""
  case "$kind" in
    plain) "$here/measure_cargo.sh" "$out" cargo test --workspace --no-fail-fast ;;
    runner)
      if [ -n "$jobs" ]; then
        "$here/measure_cargo.sh" "$out" python3 scripts/run_test_binaries.py --log-dir "$out/logs" -j "$jobs"
      else
        "$here/measure_cargo.sh" "$out" python3 scripts/run_test_binaries.py --log-dir "$out/logs"
      fi ;;
    *) echo "unknown run kind $kind" >&2; exit 2 ;;
  esac
  cargo clean >/dev/null 2>&1
  echo "$run exit $(cat "$out/exit") $(grep Elapsed "$out/time.txt" | awk '{print $NF}') free $(df -h /home | tail -1 | awk '{print $4}')"
done
