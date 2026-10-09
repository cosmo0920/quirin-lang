#!/usr/bin/env bash
# identity.sh BEFORE AFTER -- VERDICT-CONTRACT-DEPENDENCIES: the verdict-only
# projection (verdict_projection.py) and the parity comparison (parity.py) of
# two runs on the corpus, system and tail-pause slice, the dependency roll-up
# of AFTER on each, into private/vbs/{projection,parity,rollup}-AFTER-ROUTE.*.
# Each step's exit is printed; nothing is piped into a step.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/vbs"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
for route in corpus system slice; do
  case "$route" in
    corpus) b="$out/$before"; a="$out/$after"; pb="$out/$before-parity.jsonl"; pa="$out/$after-parity.jsonl";;
    system) b="$out/$before-system"; a="$out/$after-system"; pb="$out/$before-system-parity.jsonl"; pa="$out/$after-system-parity.jsonl";;
    slice) b="$slices/vbs-$before"; a="$slices/vbs-$after"; pb="$out/$before-slice-parity.jsonl"; pa="$out/$after-slice-parity.jsonl";;
  esac
  python3 "$here/verdict_projection.py" "$b" "$a" "$out/projection-$after-$route.json" > "$out/projection-$after-$route.log" 2>&1
  echo "$route projection exit $?"
  python3 "$here/parity.py" "$pb" "$pa" "$out/parity-$after-$route.json" > "$out/parity-$after-$route.log" 2>&1
  echo "$route parity exit $? $(head -1 "$out/parity-$after-$route.log")"
  python3 "$here/rollup.py" "$a" "$out/rollup-$after-$route.json" rd_kafka_toppar_purge_and_disable_queues > "$out/rollup-$after-$route.log" 2>&1
  echo "$route rollup exit $? $(head -1 "$out/rollup-$after-$route.log")"
done
