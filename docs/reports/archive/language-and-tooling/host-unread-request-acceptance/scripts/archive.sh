#!/usr/bin/env bash
# archive.sh -- HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: copy the
# campaign JSON, identity listings and route totals from private/hur into the
# report's artifacts/, and regenerate the campaign summary.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
h="$repo/private/hur"
a="$here/../artifacts"
mkdir -p "$a/campaign" || exit 2
cp "$h"/campaign/*.json "$a/campaign/" || exit 3
cp "$h/identity.txt" "$a/identity.txt" || exit 3
cp "$h/identity-rebased.txt" "$a/identity-rebased.txt" || exit 3
python3 "$here/summarize.py" "$h/campaign" base after rebased > "$a/campaign/SUMMARY.md" || exit 3
python3 "$here/totals.py" after rebased > "$a/totals.txt" || exit 3
grep -c 'DIFFERENT\|MISSING' "$a/identity.txt" "$a/identity-rebased.txt"
tail -5 "$a/campaign/SUMMARY.md"
