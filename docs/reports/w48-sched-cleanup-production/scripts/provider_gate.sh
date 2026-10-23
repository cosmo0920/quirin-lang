#!/bin/sh
# Provider gate for W48-SCHED-CLEANUP-PRODUCTION: each step's exit captured to
# a file (never piped). Usage: provider_gate.sh LOGDIR
set -u
logs=$1
mkdir -p "$logs"
step() {
  name=$1
  shift
  "$@" >"$logs/$name.log" 2>&1
  echo $? >"$logs/$name.exit"
}
export CARGO_BUILD_JOBS=6
step provider-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step provider-python python3 -m unittest discover -s providers/tests
step provider-inventory python3 providers/contracts.py check
rm -rf "$logs/packages"
step package-build python3 providers/package.py build --output "$logs/packages"
for package in "$logs"/packages/*/; do
  name=$(basename "$package")
  step "package-verify-$name" python3 providers/package.py verify "$package"
done
