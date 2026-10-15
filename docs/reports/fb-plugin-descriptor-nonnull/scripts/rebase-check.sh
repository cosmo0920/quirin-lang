#!/usr/bin/env bash
# rebase-check.sh NAME -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: the targeted check after the
# rebase onto b2fbde2c5 (integrator's list): cargo check, submodule, fmt,
# clippy --locked; the qcore-contract, qcore-c-gateway and qcore-audit test
# suites; the compiler's contract tests; check_lean_audits.py. Every step under
# the shared heavy lock, output and exit to private/fbn/rebase-NAME/, nothing
# piped; `cargo clean --profile dev` at the end (the disk rule). The provider
# gate is provider-gate.sh; the matrix, Python suite and docs index follow the
# records (finish-rebase.sh).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/fbn/rebase-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
git rev-parse HEAD > "$out/head"
step r00-check cargo check --workspace --all-targets
step r01-submodule git submodule update --init --recursive
step r02-fmt cargo fmt --all -- --check
step r03-clippy cargo clippy --locked --all-targets -- -D warnings
step r04-crate-tests cargo test --no-fail-fast -p qcore-contract -p qcore-c-gateway -p qcore-audit
step r05-compiler-contract-tests cargo test --no-fail-fast -p qcore-compiler \
  --test contract_consumers --test contract_inherit_adoption --test contract_inherit_parity \
  --test contract_inherit_qcore_provider --test contract_loader --test contract_view_census \
  --test extension_contract_compatibility --test native_library_contract --test nullable_removed \
  --test qcore_contract_corpus --test quant_index_contracts
step r06-lean python3 scripts/check_lean_audits.py
step r07-clean cargo clean --profile dev
echo done > "$out/done"
