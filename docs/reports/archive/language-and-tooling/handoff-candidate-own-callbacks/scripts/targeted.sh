#!/usr/bin/env bash
# targeted.sh NAME -- HANDOFF-CANDIDATE-OWN-CALLBACKS: the integrator's
# targeted check after the rebase onto 711287771 (librdkafka client.qv
# re-pinned there), under one exclusive flock on the shared heavy lock:
# cargo check, the gateway crate's tests, the compiler's contract tests, the
# Lean audits, the matrix regenerated with this card's gate.json, the Python
# suite after it, and the docs index and its check. Every step's output and
# exit to private/hcoc/targeted-NAME/; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/hcoc/targeted-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/head"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" /usr/bin/env bash -c '
  out="$1"
  step() { local label="$1"; shift; "$@" > "$out/$label.log" 2>&1; echo $? > "$out/$label.exit"; }
  echo "lock held from $(date -u +%FT%T.%NZ)" > "$out/lock.txt"
  step t0-check cargo check --workspace --all-targets
  step t1-gateway-tests cargo test -p qcore-c-gateway --no-fail-fast
  step t2-contract-tests cargo test -p qcore-compiler --no-fail-fast --test contract_loader --test qcore_contract_corpus --test contract_view_census
  step t3-lean python3 scripts/check_lean_audits.py
  step t4-matrix python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/handoff-candidate-own-callbacks/gate.json
  step t5-python python3 -m unittest discover -s scripts/tests
  step t6-index python3 scripts/build_docs_index.py
  step t7-index-check python3 scripts/build_docs_index.py --check
  step t8-clean cargo clean --profile dev
  echo "released at $(date -u +%FT%T.%NZ)" >> "$out/lock.txt"
' _ "$out"
grep -H . "$out"/*.exit
