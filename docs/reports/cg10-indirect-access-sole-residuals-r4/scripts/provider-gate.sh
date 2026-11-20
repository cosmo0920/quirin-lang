#!/bin/sh
# provider-gate.sh OUT -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4 (from
# round 2's provider-gate.sh): the provider gate (docs/agents/providers.md):
# cargo check/test/fmt/clippy on providers/, contracts.py check, package build
# and verify of every package, the providers Python suite. Run under the host
# toolchain (private/toolchain/env.sh); each step's output and exit go to OUT.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
packages="$out/packages"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
step() {
  label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
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
