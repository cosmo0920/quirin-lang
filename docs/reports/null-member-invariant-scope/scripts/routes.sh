#!/usr/bin/env bash
# routes.sh BEFORE AFTER -- NULL-MEMBER-INVARIANT-SCOPE: custody totals, the
# null comparison (compare.py), candidate counts (member_counts.py) and the full
# parity comparison (parity.py) of two runs on the three routes, into
# private/nmis/{compare,parity,counts}-ROUTE-AFTER.json. Nothing piped.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo/private/nmis" || exit 2
python3 "$here/totals.py" "$before" "$after" "$before-system" "$after-system"
for route in corpus system slice; do
  if [ "$route" = corpus ]; then b="$before"; a="$after"; else b="$before-$route"; a="$after-$route"; fi
  echo "== $route"
  python3 "$here/compare.py" "$b-null.jsonl" "$a-null.jsonl" "compare-$route-$after.json" > "compare-$route-$after.log"
  python3 "$here/member_counts.py" "$a-null.jsonl" "counts-$route-$after.json" > /dev/null
  python3 "$here/member_counts.py" "$b-null.jsonl" "counts-$route-$before.json" > /dev/null
  python3 "$here/parity.py" "$b-parity.jsonl" "$a-parity.jsonl" "parity-$route-$after.json" > "parity-$route-$after.log"
  tail -2 "parity-$route-$after.log"
done
