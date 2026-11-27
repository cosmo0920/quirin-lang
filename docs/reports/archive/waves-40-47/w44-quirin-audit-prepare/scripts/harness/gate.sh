#!/bin/sh
# Standing gate plus provider gate, every exit captured, nothing piped.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a0ab5a53f8a82118c
cd "$root" || exit 2
out="$root/private/gate"
mkdir -p "$out"
export CARGO_INCREMENTAL=0
export PATH="$HOME/.elan/bin:$PATH"
step() {
  name=$1
  shift
  "$@" > "$out/$name.log" 2>&1
  status=$?
  echo "$status" > "$out/$name.exit"
  echo "$name exit=$status"
}
step submodules git submodule update --init --recursive
step clang clang --version
step test python3 scripts/run_test_binaries.py
step clean cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step probe cargo run -p qcore-compiler --example reevaluation_probe
step followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py --jobs 4
step python python3 -m unittest discover -s scripts/tests
step build-offline cargo build --offline --workspace
rm -rf "$root/private/packages"
step package-build python3 providers/package.py build --output "$root/private/packages"
for package in "$root"/private/packages/*/; do
  name=$(basename "$package")
  step "package-verify-$name" python3 providers/package.py verify "$package"
done
step provider-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step contracts-check python3 providers/contracts.py check
echo "gate done"
