#!/usr/bin/env bash
# census.sh NAME -- RELATIONAL-REWRITE-LINE-ORDER: the stream line-order census
# (census.py) of run NAME on every route, into artifacts/census-NAME-ROUTE.json.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
private="$repo/private/rrlo"
artifacts="$here/../artifacts"
mkdir -p "$artifacts"
python3 "$here/census.py" "$private/$name-parity.jsonl" "$private/$name" \
  "$artifacts/census-$name-corpus.json"
python3 "$here/census.py" "$private/$name-system-parity.jsonl" "$private/$name-system" \
  "$artifacts/census-$name-system.json"
python3 "$here/census.py" "$private/$name-slice-parity.jsonl" \
  "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/rrlo-$name" \
  "$artifacts/census-$name-slice.json"
