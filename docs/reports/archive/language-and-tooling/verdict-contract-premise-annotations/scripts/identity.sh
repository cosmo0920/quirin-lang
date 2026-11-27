#!/usr/bin/env bash
# identity.sh BEFORE AFTER -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS: identity.py
# (verdict rows, dependency rows, every differing path a contract input) and
# parity.py (the typed-path parity streams, in order and as multisets) of two
# runs on the corpus, the system route and the tail-pause slice, into
# private/vcpa/{identity,parity}-AFTER-ROUTE.{json,log}. Each step's exit is
# printed; nothing is piped into a step.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/vcpa"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
for route in corpus system slice; do
  case "$route" in
    corpus) b="$out/$before"; a="$out/$after"; pb="$out/$before-parity.jsonl"; pa="$out/$after-parity.jsonl";;
    system) b="$out/$before-system"; a="$out/$after-system"; pb="$out/$before-system-parity.jsonl"; pa="$out/$after-system-parity.jsonl";;
    slice) b="$slices/vcpa-$before"; a="$slices/vcpa-$after"; pb="$out/$before-slice-parity.jsonl"; pa="$out/$after-slice-parity.jsonl";;
  esac
  python3 "$here/identity.py" "$b" "$a" "$out/identity-$after-$route.json" > "$out/identity-$after-$route.log" 2>&1
  echo "$route identity exit $? $(head -1 "$out/identity-$after-$route.log")"
  python3 "$here/parity.py" "$pb" "$pa" "$out/parity-$after-$route.json" > "$out/parity-$after-$route.log" 2>&1
  echo "$route parity exit $? $(head -1 "$out/parity-$after-$route.log")"
done
