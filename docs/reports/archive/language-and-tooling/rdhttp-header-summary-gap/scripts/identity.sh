#!/usr/bin/env bash
# identity.sh BEFORE AFTER -- RDHTTP-HEADER-SUMMARY-GAP (adapted from DECLARED-ASSUMPTION-BODY-CHECKS) (adapted from VERDICT-BODY-STANDING-CLAUSE-SCOPE) (adapted from
# VERDICT-CONTRACT-DEPENDENCIES): on the corpus, system and tail-pause slice,
# the projection without the card's keys (verdict_projection.py), the parity
# comparison (parity.py) and the clause roll-up against BEFORE
# (clause_rollup.py), into private/rhsg/{projection,parity,clauses}-AFTER-ROUTE.*.
# Each step's exit is printed; nothing is piped into a step.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rhsg"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
for route in corpus system slice; do
  case "$route" in
    corpus) b="$out/$before"; a="$out/$after"; pb="$out/$before-parity.jsonl"; pa="$out/$after-parity.jsonl";;
    system) b="$out/$before-system"; a="$out/$after-system"; pb="$out/$before-system-parity.jsonl"; pa="$out/$after-system-parity.jsonl";;
    slice) b="$slices/rhsg-$before"; a="$slices/rhsg-$after"; pb="$out/$before-slice-parity.jsonl"; pa="$out/$after-slice-parity.jsonl";;
  esac
  python3 "$here/verdict_projection.py" "$b" "$a" "$out/projection-$after-$route.json" > "$out/projection-$after-$route.log" 2>&1
  echo "$route projection exit $?"
  python3 "$here/parity.py" "$pb" "$pa" "$out/parity-$after-$route.json" > "$out/parity-$after-$route.log" 2>&1
  echo "$route parity exit $? $(head -1 "$out/parity-$after-$route.log")"
  python3 "$here/clause_rollup.py" "$b" "$a" "$out/clauses-$after-$route.json" > "$out/clauses-$after-$route.log" 2>&1
  echo "$route clauses exit $? $(head -1 "$out/clauses-$after-$route.log")"
done
