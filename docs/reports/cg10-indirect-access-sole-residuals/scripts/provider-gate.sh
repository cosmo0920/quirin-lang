#!/usr/bin/env bash
# provider-gate.sh NAME -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (from CG10-INDIRECT-ACCESS-SOLE-BLOCKERS, from CAST-FORMALIZATION) (from CONTRACT-NORETURN-AND-BORROWED-TAILS, from
# W48-BOTTOM-UP-CALLEE-BODIES): the provider gate (constraint 18). providers/
# is unchanged by this card; it is run because build_matrix.py needs a
# measured provider_tests. Outputs and exits to
# private/cg10res/gate-NAME/, each step under the shared heavy lock, nothing
# piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/cg10res/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step 10-prov-check cargo check --locked --manifest-path providers/Cargo.toml --workspace
step 11-prov-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step 12-prov-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step 13-prov-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step 13b-prov-python python3 -m unittest discover -s providers/tests
step 14-contracts python3 providers/contracts.py check
rm -rf "$out/packages"
step 15-package-build python3 providers/package.py build --output "$out/packages"
for package in "$out"/packages/qcore.*; do
  [ -d "$package" ] || continue
  step "16-verify-$(basename "$package")" python3 providers/package.py verify "$package"
done
flock "$lock" cargo clean --manifest-path providers/Cargo.toml --profile dev > "$out/prov-clean.log" 2>&1
echo done > "$out/prov-done"
