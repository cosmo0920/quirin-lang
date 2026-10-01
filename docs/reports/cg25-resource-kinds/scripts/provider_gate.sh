#!/bin/sh
# CG25-RESOURCE-KINDS: the provider gate (constraint 18; validation.md), run
# because providers/quirin-provider-libc changed. usage: provider_gate.sh LOG_DIR
# (relative to the checkout root). Every exit is captured; no step is piped.
# Packages are built into LOG_DIR/packages, never a shared /tmp path.
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
mkdir -p "$logs"
step() {
  name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
}
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step p1-check cargo check --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline
step p2-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step p3-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step p4-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step p5-package-build python3 providers/package.py build --output "$logs/packages"
step p6-package-verify python3 providers/package.py verify "$logs/packages/qcore.libevent"
step p7-contracts-check python3 providers/contracts.py check
