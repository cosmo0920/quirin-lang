#!/bin/sh
# Card CHAINED-ASSIGNMENT-INNER-DESTINATION: the census as run (read-only
# inputs in the primary checkout; output under the worktree's private/).
# Usage: run_census.sh OUT.json [ANALYSED.json]
set -e
analysed=${2:+--analysed-out=$2}
here=$(dirname "$0")
repo=$(cd "$here/../../../.." && pwd)
primary=/media/Data/GitHub/quirin-lang
base=$primary/private/baselines/7b726587bdf6a8213f451fa1c78297bb5e476ef7
cycle=$primary/build/cycle19
current=$cycle/clang/046accf6aded7906f66a1baf0202da349e521736
exec python3 "$here/census.py" \
  --tree current="$current/fluent-bit-clang-source" \
  --tree corpus-fb="$cycle/c-gateway-wave-pin/fluent-bit-clang-source" \
  --tree corpus-mk="$cycle/c-gateway-wave-pin/monkey" \
  --tree system="$primary/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source" \
  --tree slice="$primary/private/w47-tail/attempt-1/prep/fluent-bit-clang-source" \
  --contracts "$repo/providers" --contracts "$repo/contract" \
  --report current=current="$current/reports/resource-diagnostics.json" \
  --report corpus-fb=corpus-fb="$base/eight-home/corpus/corpus/resource-diagnostics.json" \
  --report corpus-mk=corpus-mk="$base/eight-home/corpus/corpus/resource-diagnostics.json" \
  --report system=system="$base/fluent-bit-system/system/resource-diagnostics.json" \
  --report slice=slice="$base/fluent-bit-tail-pause/slice/resource-diagnostics.json" \
  --out "$1" $analysed
