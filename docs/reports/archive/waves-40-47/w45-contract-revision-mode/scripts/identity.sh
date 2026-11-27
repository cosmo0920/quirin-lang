#!/usr/bin/env bash
# Compare an arbiter run with the W45-PARENTHESISED-CALLEE baseline, the
# worktree root aliased to the baseline's, the card's additive sidecar key named;
# then disclose the contract sidecar's difference (contracts_difference.py) and
# the source-effects digest-only difference (the deferred-review card's script).
# usage: identity.sh RUN OUT_PREFIX [BASELINE_DIR]      (from the repository root)
# BASELINE_DIR defaults to the W45-PARENTHESISED-CALLEE baseline the card was
# written against; after merging W45-FB-RETRY-LIFETIME the current baseline is
# docs/reports/w45-fb-retry-lifetime/integration/artifacts.
# writes OUT_PREFIX.json, OUT_PREFIX-contracts.json, OUT_PREFIX-source-effects.json
# and an .exit file beside each.
set -u
run="$1"; out="$2"
root="$(pwd)"
baseline="${3:-docs/reports/w45-parenthesised-callee/integration/artifacts}"
alias="$root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da"
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py "$run" "$baseline" \
  --root-alias "$alias" --additive-field contracts:contract_revision > "$out.json"
echo $? > "$out.json.exit"
python3 docs/reports/w45-contract-revision-mode/scripts/contracts_difference.py "$run" \
  "$baseline" "$alias" > "$out-contracts.json"
echo $? > "$out-contracts.json.exit"
python3 docs/reports/w45-contract-deferred-review/scripts/source_effects_digests.py "$run" \
  "$baseline" "$alias" > "$out-source-effects.json"
echo $? > "$out-source-effects.json.exit"
cat "$out.json.exit" "$out-contracts.json.exit" "$out-source-effects.json.exit"
