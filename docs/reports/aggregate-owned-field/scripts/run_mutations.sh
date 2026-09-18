#!/usr/bin/env bash
# Runs mutations.py detached-safely: the log goes beside the report and the
# harness's exit code to measure/mutations.exit (never through a pipe).
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 9
mkdir -p measure
python3 docs/reports/aggregate-owned-field/scripts/mutations.py \
    docs/reports/aggregate-owned-field/mutations.log > measure/mutations.out 2>&1
echo $? > measure/mutations.exit
