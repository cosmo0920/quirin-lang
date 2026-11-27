#!/usr/bin/env bash
# artifacts.sh BEFORE AFTER -- OP-CONSTRUCTOR-DECLARATIONS: the report's
# artifacts for one BEFORE/AFTER pair of runs under private/opcand/ (and the
# slice runs under the shared prepared attempt): the three route compares, the
# null-property row comparison, the member sizes and the summary. Outputs go
# to artifacts/; console copies to private/opcand/artifacts-*.log.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
art="$here/../artifacts"
priv="$repo/private/opcand"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
python3 "$here/compare.py" "$priv/$before" "$priv/$after" "$art/compare-corpus.json" > "$priv/artifacts-corpus.log" 2>&1
python3 "$here/compare.py" "$priv/$before-system" "$priv/$after-system" "$art/compare-system.json" > "$priv/artifacts-system.log" 2>&1
python3 "$here/compare.py" "$slice/opcand-$before" "$slice/opcand-$after" "$art/compare-slice.json" > "$priv/artifacts-slice.log" 2>&1
python3 "$here/null_compare.py" "$before-null" "$after-null" "$before-system-null" "$after-system-null" \
  "$before-slice-null" "$after-slice-null" > "$art/null-compare.txt" 2>&1
python3 "$here/size_members.py" "$priv/$after" "$art/size-after-corpus.json" > "$priv/artifacts-size.log" 2>&1
python3 "$here/summarize.py" "$before" "$after" > "$priv/artifacts-summary.log" 2>&1
echo "exits: $?"
head -1 "$priv/artifacts-corpus.log" "$priv/artifacts-system.log" "$priv/artifacts-slice.log"
cat "$art/null-compare.txt"
