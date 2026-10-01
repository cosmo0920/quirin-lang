#!/usr/bin/env bash
# slice.sh NAME -- the tail-pause slice alone (measure.sh's third step) with the
# release binary in target/release, as cg22-NAME under the shared prepared
# attempt. Used after attempt `after` was stopped when the host disk filled.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22"
cd "$repo" || exit 2
QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl" \
  bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cg22-$name" \
  "$repo/target/release/quirin-audit" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
