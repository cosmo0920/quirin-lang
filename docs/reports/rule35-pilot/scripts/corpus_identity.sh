#!/bin/sh
# RULE35-PILOT: the corpus route's identity beyond the tool's byte comparison
# (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY: each side's unit build identity
# digests its own working directory). Runs GATEWAY-RULE-API's two checkers on
# every corpus artifact the tool named as differing, BASE against TIP:
# substitution.py (the texts differ only by a one-to-one SHA-256 substitution)
# and identity_check.py (equal with digests blanked, and the per-function
# build identities map one to one, one per unit).
# Usage: corpus_identity.sh WORKTREE MEASURE_OUT OUT_PREFIX
set -u
W=$1
S=$2/sides
OUT=$3
cd "$W" || exit 2
B=$S/base/corpus
T=$S/tip/corpus
python3 docs/reports/gateway-rule-api/scripts/substitution.py \
  "$B/corpus-null.jsonl" "$T/corpus-null.jsonl" \
  "$B/corpus/resource-diagnostics.components.json" "$T/corpus/resource-diagnostics.components.json" \
  "$B/corpus/resource-diagnostics.contracts.json" "$T/corpus/resource-diagnostics.contracts.json" \
  "$B/corpus/resource-diagnostics.json" "$T/corpus/resource-diagnostics.json" \
  "$B/corpus/resource-diagnostics.source-effects.json" "$T/corpus/resource-diagnostics.source-effects.json" \
  >"$OUT-substitution.jsonl"
echo $? >"$OUT-substitution.exit"
python3 docs/reports/gateway-rule-api/scripts/identity_check.py "$B" "$T" \
  corpus-null.jsonl corpus-null.summary.json corpus-outer.json \
  corpus/monkey-source.json corpus/resource-diagnostics.chunk-contracts.json \
  corpus/resource-diagnostics.components.json corpus/resource-diagnostics.contracts.json \
  corpus/resource-diagnostics.json corpus/resource-diagnostics.source-effects.json \
  >"$OUT-check.jsonl"
echo $? >"$OUT-check.exit"
python3 docs/reports/rule35-pilot/scripts/substitution_ids.py \
  "$B/corpus/resource-diagnostics.components.json" "$T/corpus/resource-diagnostics.components.json" \
  "$B/corpus/resource-diagnostics.json" "$T/corpus/resource-diagnostics.json" \
  "$B/corpus/resource-diagnostics.source-effects.json" "$T/corpus/resource-diagnostics.source-effects.json" \
  >"$OUT-substitution-ids.jsonl"
echo $? >"$OUT-substitution-ids.exit"
