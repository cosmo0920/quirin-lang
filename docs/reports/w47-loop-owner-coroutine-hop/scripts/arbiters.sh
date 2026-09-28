#!/usr/bin/env bash
# arbiters.sh OUT COMMIT -- the three measurements this card must show
# unchanged, each into a fresh directory, every exit captured to a file:
# the 185-unit corpus (quirin-audit pinned at 66910c10), the Fluent Bit
# system slice at 046accf6, and the tail-pause slice at the 90978ac6 export.
# Uses target/release/quirin-audit as built by the caller; read-only on every
# source tree.
set -u
out="$1"; commit="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
sha256sum target/release/quirin-audit > "$out/binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/corpus/parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/corpus" "$commit" --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 8 > "$out/corpus.log" 2>&1
echo $? > "$out/corpus.exit"
python3 docs/reports/w46-contract-gap-expansion/scripts/run_system.py "$out/system" --jobs 8 \
  > "$out/system.log" 2>&1
echo $? > "$out/system.exit"
run="hop-$(basename "$out")"
bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "$run" \
  "$repo/target/release/quirin-audit" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
  > "$out/slice.log" 2>&1
echo $? > "$out/slice.exit"
echo "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/$run" > "$out/slice.path"
