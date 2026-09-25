#!/bin/sh
# Rerun RUN's src-includes configuration with --jobs 1 and compare its report
# and sidecar with the 16-job run byte for byte. usage: jobs-check.sh RUN
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa974c0228e7d5c45
cd "$root" || exit 2
run="$root/private/measure/${1:?usage: jobs-check.sh RUN}"
out="$run/jobs1"
if [ -e "$out" ]; then
  echo "$out exists" >&2
  exit 2
fi
QCORE_CLANG=$(command -v clang)
export QCORE_CLANG
# Both forms must agree on the report, so the one-job run gets its own
# configuration: the same settings with another report path and one job.
python3 -c 'import json, sys; c = json.load(open(sys.argv[1])); c["report"] = sys.argv[2]; c["jobs"] = 1; json.dump(c, open(sys.argv[3], "w"), indent=1)' \
  "$run/src-includes.json" "$out/report.json" "$run/src-includes-jobs1.json"
"$root/target/release/quirin-audit" contracts --config "$run/src-includes-jobs1.json" \
  > "$run/jobs1.log" 2>&1
echo $? > "$run/jobs1.exit"
cmp "$run/src-includes/report.json" "$out/report.json" > "$run/jobs1-report.cmp" 2>&1
echo $? > "$run/jobs1-report.cmp.exit"
cmp "$run/src-includes/report.contracts.json" "$out/report.contracts.json" > "$run/jobs1-sidecar.cmp" 2>&1
echo $? > "$run/jobs1-sidecar.cmp.exit"
echo "jobs1 exit=$(cat "$run/jobs1.exit") report cmp=$(cat "$run/jobs1-report.cmp.exit") sidecar cmp=$(cat "$run/jobs1-sidecar.cmp.exit")"
