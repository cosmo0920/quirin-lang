#!/usr/bin/env bash
# classify-run.sh RUN STAGE -- FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: the chain
# census (chain_blockers_census.py) and its classification (classify.py) of
# one census run RUN (census-run.sh's NAME) for both routes, into
# private/fblb/STAGE-{chain,classes}-{system,slice}.json. Read-only.
set -u
run="$1"; stage="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fblb"
system_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
slice_root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
slice_report=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/fblb-$run/resource-diagnostics.json
python3 "$here/chain_blockers_census.py" "system-$stage" "$out/$run-system/resource-diagnostics.json" \
  "$system_root" "$out/$stage-chain-system.json" "$out/$run-system-census.jsonl" || exit 1
python3 "$here/chain_blockers_census.py" "slice-$stage" "$slice_report" \
  "$slice_root" "$out/$stage-chain-slice.json" "$out/$run-slice-census.jsonl" || exit 1
python3 "$here/classify.py" "$out/$run-system-records.jsonl" "$system_root" \
  "$out/$stage-chain-system.json" "$out/$stage-classes-system.json" || exit 1
python3 "$here/classify.py" "$out/$run-slice-records.jsonl" "$slice_root" \
  "$out/$stage-chain-slice.json" "$out/$stage-classes-slice.json" || exit 1
