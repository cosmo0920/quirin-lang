#!/usr/bin/env bash
# provider-gate.sh NAME -- NONNULL-MEMBER-UNIFIED-QUALIFIER (from FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE, from NULL-MEMBER-INVARIANT-SCOPE): the provider gate
# (docs/agents/providers.md; brief-callee-closure-r3.md "Contract placement"):
# cargo test/fmt/clippy and `cargo check --locked` on providers/, contracts.py
# check, package build and verify of every package, and the providers Python
# suite. Each step under the shared heavy lock, output and exit to
# private/nmuq/provider-gate-NAME/; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/nmuq/provider-gate-$name"
packages="$out/packages"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
  echo "$label $(cat "$out/$label.exit")"
}
git rev-parse HEAD > "$out/head"
step p1-check cargo check --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline
step p2-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step p3-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step p4-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step p5-contracts python3 providers/contracts.py check
rm -rf "$packages"
step p6-package-build python3 providers/package.py build --output "$packages"
for package in "$packages"/*; do
  [ -d "$package" ] || continue
  step "p7-verify-$(basename "$package")" python3 providers/package.py verify "$package"
done
step p8-unittest python3 -m unittest discover -s providers/tests
echo done > "$out/done"
