#!/bin/sh
# W47-ALIAS-RESIDUALS-CENSUS: the whole census from the two census runs.
# usage: pipeline.sh CENSUS_DIR BEFORE_DIR WORK_DIR
#   CENSUS_DIR  run_census.py outputs `corpus/` and `system/` (indirect-census build)
#   BEFORE_DIR  the default-build BEFORE runs `corpus/` and `system/` (the reports classified)
#   WORK_DIR    private output directory
# Every exit is captured to a file. publish.py copies the small evidence.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
census="$1"; before="$2"; work="$3"
mkdir -p "$work"
step() {
  name="$1"; shift
  "$@" > "$work/$name.log" 2>&1
  echo $? > "$work/$name.exit"
  echo "$name $(cat "$work/$name.exit")"
}
for scope in corpus system; do
  step "census-$scope" python3 "$here/alias_census.py" "$census/$scope" \
    "$before/$scope/resource-diagnostics.json" "$before/$scope/resource-diagnostics.contracts.json" \
    "$work/$scope" --jobs 4 --parity "$census/$scope/parity.jsonl"
  flag=""
  [ "$scope" = system ] && flag="--system"
  for key in class group; do
    step "functions-$scope-$key" python3 "$here/functions.py" "$before/$scope/resource-diagnostics.json" \
      "$work/$scope" "$work/$scope/functions-$key.json" --key "$key" $flag
  done
done
step publish python3 "$here/publish.py" "$work" "$here/../artifacts/census"
