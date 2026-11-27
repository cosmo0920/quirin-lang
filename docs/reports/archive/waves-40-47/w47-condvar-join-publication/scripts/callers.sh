#!/usr/bin/env bash
# callers.sh RUNS_DIR OUT_DIR -- scripts/callers.py over one corpus, system
# and slice run (RUNS_DIR/{corpus,system,slice}) against their source roots:
# the 66910c10 corpus source, the prepared 046accf6 clone and the 90978ac6
# export (synthetic commit 34155955). Read-only on every source.
set -u
runs="$1"
out="$2"
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$out" || exit 2
python3 "$here/callers.py" "$runs/corpus" \
  /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source \
  > "$out/callers-corpus.json" || exit 1
python3 "$here/callers.py" "$runs/system" \
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source \
  > "$out/callers-system.json" || exit 1
python3 "$here/callers.py" "$runs/slice" \
  /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source \
  > "$out/callers-slice.json" || exit 1
