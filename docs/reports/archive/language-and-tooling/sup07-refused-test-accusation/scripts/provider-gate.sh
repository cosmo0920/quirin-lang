#!/usr/bin/env bash
# provider-gate.sh NAME -- the provider gate (constraint 18), run although
# providers/ is unchanged because build_matrix.py merges only a gate with
# provider_tests; outputs and exits to private/sup07/gate-NAME/, each step
# under the shared heavy lock, nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/sup07/gate-$name"
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
step 14-contracts python3 providers/contracts.py check
rm -rf "$out/packages"
step 15-package-build python3 providers/package.py build --output "$out/packages"
for package in "$out"/packages/qcore.*; do
  [ -d "$package" ] || continue
  step "16-verify-$(basename "$package")" python3 providers/package.py verify "$package"
done
flock "$lock" cargo clean --manifest-path providers/Cargo.toml --profile dev > "$out/prov-clean.log" 2>&1
echo done > "$out/prov-done"
