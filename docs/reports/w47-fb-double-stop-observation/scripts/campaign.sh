#!/usr/bin/env bash
# campaign.sh TREE VARIANT COUNT RUN_NAME -- COUNT runs of each scenario
# (control, v, i, iii) into attempt-1/RUN_NAME, then summarize.py into
# RUN_NAME/summary.json (the aggregate is printed).
set -u
tree="$1"; variant="$2"; count="$3"; name="$4"
here="$(cd "$(dirname "$0")" && pwd)"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1
run="$root/$name"
[ -e "$run" ] && { echo "refusing to reuse $run"; exit 2; }
for s in control v i iii; do
  printf '%s: ' "$s"
  bash "$here/run.sh" "$tree" "$variant" "$s" "$count" "$run" 2>> "$run.driver-stderr"
done
python3 "$here/summarize.py" "$run" "$root/build-$tree-$variant/lib/libfluent-bit.so" "$variant" "$run/summary.json"
echo "summarize exit $?"
