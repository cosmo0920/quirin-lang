#!/usr/bin/env bash
# parity-all.sh BEFORE AFTER -- W45-CALLEE-CASCADE-RESIDUAL round 3 (from CG22-CALLBACK-REGISTRY-CONTRACT and its lineage): scripts/parity.py over
# the corpus, system and tail-pause slice parity streams of two runs, each
# against the moving rows compare.py found (private/w45r3/compare-AFTER-*.json,
# written first by this script), into private/w45r3/parity-AFTER-*.{json,log}.
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/w45r3"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
cd "$repo" || exit 2
python3 "$here/compare.py" "$out/$before" "$out/$after" "$out/compare-$after-corpus.json" \
  > "$out/compare-$after-corpus.log" 2>&1
python3 "$here/compare.py" "$out/$before-system" "$out/$after-system" "$out/compare-$after-system.json" \
  > "$out/compare-$after-system.log" 2>&1
python3 "$here/compare.py" "$slices/w45r3-$before" "$slices/w45r3-$after" "$out/compare-$after-slice.json" \
  > "$out/compare-$after-slice.log" 2>&1
for route in corpus system slice; do
  case "$route" in
    corpus) b="$out/$before-parity.jsonl"; a="$out/$after-parity.jsonl";;
    *) b="$out/$before-$route-parity.jsonl"; a="$out/$after-$route-parity.jsonl";;
  esac
  python3 "$here/parity.py" "$b" "$a" "$out/parity-$after-$route.json" "$out/compare-$after-$route.json" \
    > "$out/parity-$after-$route.log" 2>&1
  echo "$route $(tail -2 "$out/parity-$after-$route.log" | head -1)"
done
