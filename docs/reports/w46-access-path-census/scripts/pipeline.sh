#!/usr/bin/env bash
# W46-ACCESS-PATH-CENSUS: the whole census from one census run.
# usage: pipeline.sh RUN_DIR WORK_DIR
#   RUN_DIR  a run_census.py output (census-feature build; artifacts identical to the baseline)
#   WORK_DIR private output directory (large intermediates)
# Every exit is captured to a file; the report directory receives only the
# small published evidence (publish.py).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
base="$(cd "$here/../../w46-external-callee-contracts/integration/artifacts" && pwd)"
run="$1"
work="$2"
mkdir -p "$work"
export PYTHONHASHSEED=0
step() {
  local name="$1"; shift
  "$@" > "$work/$name.log" 2>&1
  echo $? > "$work/$name.exit"
  echo "$name $(cat "$work/$name.exit")"
}
step census python3 "$here/access_census.py" "$run" "$base/diagnostics.json.gz" "$base/contracts.json.gz" "$work/census" --jobs 6
# The join refuses to publish on any unmatched row (exit 3, census/unmatched.json).
[ "$(cat "$work/census.exit")" = 0 ] || { echo "join refused: see $work/census/unmatched.json"; exit 3; }
step predict python3 "$here/predict.py" "$work/census" "$run" "$base/diagnostics.json.gz" "$base/contracts.json.gz" "$work/predict"
step report python3 "$here/report.py" "$work/census" "$work/predict" "$run" "$base/diagnostics.json.gz" "$base/contracts.json.gz" "$work/census.json" "$work/report.txt"
step registered bash "$here/registered_classes.sh" "$run" "$work/classes"
step classes python3 "$here/classes.py" "$work/classes" "$run" "$work/census" "$work/classes.json" "$work/classes.txt"
step publish python3 "$here/publish.py" "$work/census" "$work/predict" "$work/census.json" "$work/classes.json" "$report"
