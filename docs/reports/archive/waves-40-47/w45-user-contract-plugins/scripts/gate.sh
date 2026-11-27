#!/usr/bin/env bash
# The standing gate and the provider gate, each step's output and exit
# captured to its own file under OUT (worktree-private), nothing piped.
# usage: gate.sh OUT      (from the repository root)
set -u
out="$1"
mkdir -p "$out"
step() {
  local name="$1"; shift
  "$@" > "$out/$name.log" 2>&1
  echo $? > "$out/$name.exit"
}
export CARGO_INCREMENTAL=0
step submodules git submodule update --init --recursive
step clang clang --version
step test python3 scripts/run_test_binaries.py
step clean cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step probe cargo run -p qcore-compiler --example reevaluation_probe
step followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step contracts python3 providers/contracts.py check
step provider-python python3 -m unittest discover -s providers/tests
step packages python3 providers/package.py build --output "$out/packages"
for package in "$out"/packages/qcore.*; do
  [ -d "$package" ] || continue
  step "verify-$(basename "$package")" python3 providers/package.py verify "$package"
done
step provider-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
for file in "$out"/*.exit; do echo "$(basename "$file" .exit) $(cat "$file")"; done > "$out/exits.txt"
