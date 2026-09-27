#!/usr/bin/env bash
# W46-ACCESS-PATH-CENSUS: re-measure the registered follow-up classes on this
# baseline with the scripts that registered them, unchanged, over one census
# run of this card (run_census.py). usage: registered_classes.sh RUN_DIR OUT_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
reports="$(cd "$here/../.." && pwd)"
run="$1"
out="$2"
mkdir -p "$out"
python3 "$reports/w44-indirect-chain-access/scripts/chain_census.py" "$run" "$out/chain.json" "$out/chain.txt" > "$out/chain.log" 2>&1
echo "chain_census $?"
python3 "$reports/w43-indirect-access-general/scripts/residuals.py" "$run" "$out/w43.json" "$out/w43.txt" > "$out/w43.log" 2>&1
echo "w43 residuals $?"
gzip -c "$run/condition-call-census.jsonl" > "$out/condition-call-census.jsonl.gz"
python3 "$reports/w44-branch-condition-calls/scripts/residuals.py" "$run/resource-diagnostics.json" \
  "$out/condition-call-census.jsonl.gz" "$run/resource-diagnostics.json" "$out/condition.json" "$out/condition.txt" \
  > "$out/condition.log" 2>&1
echo "condition residuals $?"
python3 "$reports/w44-indirect-call-argument-access/scripts/family_residuals.py" "$run" "$out/call-argument.json" \
  "$out/call-argument.txt" > "$out/call-argument.log" 2>&1
echo "call-argument residuals $?"
