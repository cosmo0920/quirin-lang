#!/usr/bin/env bash
# targeted.sh NAME -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the targeted check
# after a rebase whose conflicts were records only (landing-race policy):
# cargo check, fmt, clippy, this card's tests and the crates it touches
# (qcore-contract, the gateway's null tests, qcore-audit's library and
# member-prepass route tests, the compiler's contract corpus, loader, view and
# inheritance tests), the Lean audits, the matrix regenerated with gate.json,
# the Python suite and the docs index, under one exclusive flock on the shared
# heavy lock; then the dev profile is cleaned. Exits to
# private/lppr/targeted-NAME/; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/lppr/targeted-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
if [ "${HELD:-0}" != 1 ]; then
  HELD=1 exec flock "$lock" "$0" "$@"
fi
git rev-parse HEAD > "$out/head"
step 00-check cargo check --workspace --all-targets
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --locked --all-targets -- -D warnings
step 04a-contract cargo test -p qcore-contract
step 04b-gateway cargo test -p qcore-c-gateway --test null_member_premise --test null_member_invariant_scope --test null_candidate_triage
step 04c-audit cargo test -p qcore-audit --lib
step 04d-audit-routes cargo test -p qcore-audit --test member_prepass_routes
step 04e-compiler cargo test -p qcore-compiler --test qcore_contract_corpus --test contract_loader --test contract_view_census --test contract_inherit_parity --test contract_inherit_qcore_provider
step 07-lean python3 scripts/check_lean_audits.py
step 09a-matrix python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/librdkafka-plugin-premise-remainder/gate.json
step 09b-python python3 -m unittest discover -s scripts/tests
step 10b-index python3 scripts/build_docs_index.py
step 10c-index-check python3 scripts/build_docs_index.py --check
step 07z-clean cargo clean --profile dev
grep -H . "$out"/*.exit
