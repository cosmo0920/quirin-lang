#!/usr/bin/env bash
# census-all.sh NAME -- census.py and near.py over the three routes of one
# measure.sh run (private/cnbt/NAME*, the slice under
# private/w47-tail/attempt-1/cnbt-NAME) into artifacts/*-NAME-*.json, plus the
# merged totals. Exits are not piped away: each tool's exit is printed.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
art="$here/../artifacts"
run="$repo/private/cnbt"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/cnbt-$name
mkdir -p "$art"
cd "$repo" || exit 2
python3 "$here/merged_totals.py" "$run/$name" "$run/$name-system" "$slice" > "$art/totals-$name.txt"
echo "totals $?"
python3 "$here/census.py" "$run/$name" "$run/$name-parity.jsonl" "$art/census-$name-corpus.json" > "$art/census-$name-corpus.txt"
echo "census corpus $?"
python3 "$here/census.py" "$run/$name-system" "$run/$name-system-parity.jsonl" "$art/census-$name-system.json" --route system > "$art/census-$name-system.txt"
echo "census system $?"
python3 "$here/census.py" "$slice" "$run/$name-slice-parity.jsonl" "$art/census-$name-slice.json" --route slice > "$art/census-$name-slice.txt"
echo "census slice $?"
python3 "$here/near.py" "$run/$name" "$art/near-$name-corpus.json" > "$art/near-$name-corpus.txt"
echo "near corpus $?"
python3 "$here/near.py" "$run/$name-system" "$art/near-$name-system.json" > "$art/near-$name-system.txt"
echo "near system $?"
python3 "$here/near.py" "$slice" "$art/near-$name-slice.json" > "$art/near-$name-slice.txt"
echo "near slice $?"
