#!/usr/bin/env bash
# Compare an arbiter run with the current baseline (W45-FB-RETRY-LIFETIME's
# integration artifacts), the worktree root aliased to the baseline's, with
# W45-CONTRACT-REVISION-MODE's additive sidecar key named; then disclose the
# contract sidecar's difference (this card's contracts_difference.py, which
# reuses the revision-mode card's restoration) and the source-effects
# digest-only difference (the deferred-review card's script).
# usage: identity.sh RUN OUT_PREFIX      (from the repository root)
# writes OUT_PREFIX.json, OUT_PREFIX-contracts.json, OUT_PREFIX-source-effects.json
# and an .exit file beside each.
set -u
run="$1"; out="$2"
root="$(pwd)"
baseline="docs/reports/w45-fb-retry-lifetime/integration/artifacts"
alias="$root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da"
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py "$run" "$baseline" \
  --root-alias "$alias" --additive-field contracts:contract_revision > "$out.json"
echo $? > "$out.json.exit"
python3 docs/reports/w45-user-contract-plugins/scripts/contracts_difference.py "$run" \
  "$baseline" "$alias" > "$out-contracts.json"
echo $? > "$out-contracts.json.exit"
python3 docs/reports/w45-contract-deferred-review/scripts/source_effects_digests.py "$run" \
  "$baseline" "$alias" > "$out-source-effects.json"
echo $? > "$out-source-effects.json.exit"
cat "$out.json.exit" "$out-contracts.json.exit" "$out-source-effects.json.exit"
