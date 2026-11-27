#!/bin/bash
# Card CI-RUST-JOB-PEAK: the Linux `rust` job of .github/workflows/ci.yml, step
# by step, in one sequence, on a fresh clone.
#
#   ci_rust_job.sh <clone dir> <temp root> <out dir>
#
# The clone must already be checked out with its submodule (the job's
# `actions/checkout` step) and hold nothing else.  The steps are read from the
# clone's own workflow (`jobs.rust.steps`, every step with a `run:`), so a
# before and an after run each execute their revision's job; `uses:` steps
# (checkout, toolchain installation) are not run here and are listed in
# `steps.tsv` as skipped.  Each `run:` body runs as GitHub's default bash shell
# does (`bash --noprofile --norc -eo pipefail`) in the clone, with the job's
# `env:` and fresh directories under <temp root>: `tmp/` (TMPDIR),
# `runner-temp/` (RUNNER_TEMP) and `cargo-home/` (CARGO_HOME: the registry
# starts empty, as on a fresh runner, so the crates the job downloads count).
# A failing step is recorded and the sequence continues, so one run measures
# the whole job.  `steps.tsv` has each step's start, end, exit and the size of
# every watched directory at its end; measure_peak.py around this script
# samples the peak.
set -u
CLONE=$(realpath "$1")
TEMP_ROOT=$(realpath "$2")
OUT=$(realpath "$3")
HERE=$(dirname "$(realpath "$0")")
mkdir -p "$TEMP_ROOT/tmp" "$TEMP_ROOT/runner-temp" "$TEMP_ROOT/cargo-home" "$OUT/steps"
export TMPDIR="$TEMP_ROOT/tmp"
export RUNNER_TEMP="$TEMP_ROOT/runner-temp"
export CARGO_HOME="$TEMP_ROOT/cargo-home"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
unset CARGO_TARGET_DIR RUSTFLAGS CARGO_INCREMENTAL
python3 "$HERE/workflow_steps.py" "$CLONE/.github/workflows/ci.yml" rust "$OUT/steps" > "$OUT/steps/index.tsv" || exit 2
# The job's env: block, one `export NAME=value` per line.
source "$OUT/steps/env.sh"
START=$(date +%s)
WATCH=("$CLONE" "$CLONE/target" "$CLONE/providers/target" "$TMPDIR" "$RUNNER_TEMP" "$CARGO_HOME")
size() { if [ -e "$1" ]; then du -s --block-size=1 "$1" 2>/dev/null | tail -1 | cut -f1; else echo 0; fi; }
printf 'step\tname\tstart_s\tend_s\texit' > "$OUT/steps.tsv"
for dir in "${WATCH[@]}"; do printf '\t%s' "$dir" >> "$OUT/steps.tsv"; done
printf '\n' >> "$OUT/steps.tsv"

while IFS=$'\t' read -r index kind name; do
  begin=$(( $(date +%s) - START ))
  if [ "$kind" = uses ]; then
    printf '%s\t%s\t%s\t%s\tskipped' "$index" "$name" "$begin" "$begin" >> "$OUT/steps.tsv"
  else
    echo "=== step $index $name (t=${begin}s)"
    (cd "$CLONE" && bash --noprofile --norc -eo pipefail "$OUT/steps/$index.sh") > "$OUT/steps/$index.log" 2>&1
    code=$?
    end=$(( $(date +%s) - START ))
    echo "=== step $index exit $code (t=${end}s)"
    printf '%s\t%s\t%s\t%s\t%s' "$index" "$name" "$begin" "$end" "$code" >> "$OUT/steps.tsv"
  fi
  for dir in "${WATCH[@]}"; do printf '\t%s' "$(size "$dir")" >> "$OUT/steps.tsv"; done
  printf '\n' >> "$OUT/steps.tsv"
done < "$OUT/steps/index.tsv"
