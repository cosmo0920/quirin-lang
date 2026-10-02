#!/usr/bin/env bash
# rank_all.sh NAME LABEL -- run closure_r3.py over the three routes of one
# measure.sh run (corpus and system under private/w48bu/NAME*, the slice under
# the shared prepared attempt as w48bu-NAME) into artifacts/ranking/ROUTE-LABEL.*
set -u
name="$1"
label="$2"
here="$(cd "$(dirname "$0")" && pwd)"
report="$(cd "$here/.." && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$report/artifacts/ranking"
mkdir -p "$out"
decisions="$repo/docs/reports/w48-callee-closure-r3/decisions.json"
run() {
  python3 "$here/closure_r3.py" "$2" "$out/$1-$label.json" --route "$1" --decisions "$decisions" \
    > "$out/$1-$label.txt"
  echo "$1 $?"
}
run corpus "$repo/private/w48bu/$name"
run system "$repo/private/w48bu/$name-system"
run slice "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/w48bu-$name"
