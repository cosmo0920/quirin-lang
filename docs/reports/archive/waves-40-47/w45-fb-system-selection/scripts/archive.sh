#!/usr/bin/env bash
# Copy the small, verdict-bearing outputs of one attempt into the report.
# usage: archive.sh ATTEMPT_DIR REPORT_ARTIFACTS_DIR
set -eu
attempt="$1"; out="$2"
mkdir -p "$out/system-slice" "$out/main-profile"
slice="$attempt/system-slice"
gzip -n -9 -c "$slice/resource-diagnostics.json" > "$out/system-slice/resource-diagnostics.json.gz"
gzip -n -9 -c "$slice/resource-diagnostics.contracts.json" > "$out/system-slice/resource-diagnostics.contracts.json.gz"
for name in resource-diagnostics.invocation.json console.log exit stderr.log; do
  cp "$slice/$name" "$out/system-slice/$name"
done
main="$attempt/main-profile-475bb547"
for name in resource-diagnostics.invocation.json console.log exit stderr.log; do
  cp "$main/$name" "$out/main-profile/$name"
done
