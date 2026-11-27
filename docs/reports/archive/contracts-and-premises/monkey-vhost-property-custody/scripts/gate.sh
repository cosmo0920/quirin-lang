#!/usr/bin/env bash
# gate.sh NAME -- MONKEY-VHOST-PROPERTY-CUSTODY (from CONTRACT-PIN-TIP-MODE and
# VA-LOOP-AND-POINTER-ARG): cargo check, the standing per-change gate
# (docs/agents/validation.md) and the provider gate (providers/ changed), every
# step's output and exit to private/mvp/gate-NAME/, nothing piped. Run under
# the shared heavy lock (the caller wraps it in flock).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mvp/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step 00-check cargo check --workspace --all-targets
step 01-submodule git submodule update --init --recursive
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --locked --all-targets -- -D warnings
step 04-tests python3 scripts/run_test_binaries.py
step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
step 07-lean python3 scripts/check_lean_audits.py
clang --version > "$out/clang-version.txt" 2>&1
echo $? > "$out/clang-version.exit"
step 08-python python3 -m unittest discover -s scripts/tests
step 09-index python3 scripts/build_docs_index.py --check
# Provider gate (constraint 18; providers/ changed).
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
cargo clean --profile dev > "$out/clean.log" 2>&1
