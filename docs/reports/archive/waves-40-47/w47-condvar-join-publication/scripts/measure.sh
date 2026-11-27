#!/usr/bin/env bash
# measure.sh PHASE -- the three arbiter measurements of card
# W47-CONDVAR-JOIN-PUBLICATION with this worktree's target/release/quirin-audit,
# each into a fresh directory (the commands of the common brief):
#   corpus: scripts/run_pinned_audit.py private/cj/PHASE/corpus (66910c10, 185 units)
#   system: docs/reports/w46-contract-gap-expansion/scripts/run_system.py private/cj/PHASE/system (046accf6)
#   slice:  docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 cj-PHASE
#           (the 90978ac6 export, tail-pause-selection.json)
# Every exit is written to a file; nothing is piped.
set -u
phase="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cj/$phase"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
sha256sum target/release/quirin-audit > "$out/binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/corpus/parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/corpus" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 8 > "$out/corpus.log" 2>&1
echo $? > "$out/corpus.exit"
python3 docs/reports/w46-contract-gap-expansion/scripts/run_system.py "$out/system" --jobs 8 \
  > "$out/system.log" 2>&1
echo $? > "$out/system.exit"
bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cj-$phase" \
  "$repo/target/release/quirin-audit" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
  > "$out/slice.log" 2>&1
echo $? > "$out/slice.exit"
