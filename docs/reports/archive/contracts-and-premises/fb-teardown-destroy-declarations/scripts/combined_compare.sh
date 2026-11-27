#!/usr/bin/env bash
# combined_compare.sh LABEL -- FB-TEARDOWN-DESTROY-DECLARATIONS: compare the
# combined arbiter's main runs (private/ftd/src-combined, NAME mainc) with the
# tip runs (private/ftd, NAME tipc) on the three routes, custody rows
# (compare.py) and null streams (null_compare.py), into
# artifacts/combined-LABEL-*.json, and record the binary and report
# identities in artifacts/combined-LABEL-runs.json. Prints the summaries.
set -u
label="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
art="$repo/docs/reports/fb-teardown-destroy-declarations/artifacts"
m="$repo/private/ftd/src-combined/private/ftd"
t="$repo/private/ftd"
s=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
for x in "$m/mainc.exit" "$m/mainc-system.exit" "$m/mainc-slice.exit" \
         "$t/tipc.exit" "$t/tipc-system.exit" "$t/tipc-slice.exit"; do
  echo "$(basename "$x"): $(cat "$x")"
done
python3 "$here/compare.py" "$m/mainc" "$t/tipc" "$art/combined-$label-corpus.json"
python3 "$here/compare.py" "$m/mainc-system" "$t/tipc-system" "$art/combined-$label-system.json"
python3 "$here/compare.py" "$s/ftd-mainc" "$s/ftd-tipc" "$art/combined-$label-slice.json"
python3 "$here/null_compare.py" "$m/mainc-null.jsonl" "$t/tipc-null.jsonl" "$art/combined-$label-null-corpus.json"
python3 "$here/null_compare.py" "$m/mainc-system-null.jsonl" "$t/tipc-system-null.jsonl" "$art/combined-$label-null-system.json"
python3 "$here/null_compare.py" "$m/mainc-slice-null.jsonl" "$t/tipc-slice-null.jsonl" "$art/combined-$label-null-slice.json"
{
  echo "{"
  echo " \"main_rev\": \"$(cat "$t/combined-main.rev")\","
  echo " \"tip_rev\": \"$(cat "$t/build-tipc.head")\","
  echo " \"main_binary\": \"$(cut -d' ' -f1 "$m/bin/mainc.sha256")\","
  echo " \"tip_binary\": \"$(cut -d' ' -f1 "$t/bin/tipc.sha256")\","
  echo " \"reports\": {"
  echo "  \"main_corpus\": \"$(sha256sum "$m/mainc/resource-diagnostics.json" | cut -d' ' -f1)\","
  echo "  \"tip_corpus\": \"$(sha256sum "$t/tipc/resource-diagnostics.json" | cut -d' ' -f1)\","
  echo "  \"main_system\": \"$(sha256sum "$m/mainc-system/resource-diagnostics.json" | cut -d' ' -f1)\","
  echo "  \"tip_system\": \"$(sha256sum "$t/tipc-system/resource-diagnostics.json" | cut -d' ' -f1)\","
  echo "  \"main_slice\": \"$(sha256sum "$s/ftd-mainc/resource-diagnostics.json" | cut -d' ' -f1)\","
  echo "  \"tip_slice\": \"$(sha256sum "$s/ftd-tipc/resource-diagnostics.json" | cut -d' ' -f1)\""
  echo " }"
  echo "}"
} > "$art/combined-$label-runs.json"
cat "$art/combined-$label-runs.json"
