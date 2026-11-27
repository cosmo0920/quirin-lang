#!/usr/bin/env bash
# gate.sh NAME -- the standing per-change gate (docs/agents/validation.md) and
# the provider gate (providers/ changed), every step's output and exit to
# private/contract-pin-tip-mode/gate-NAME/ (exits, head and the Clang recording
# are copied into the report's gate/ directory), nothing piped. Run under the
# shared heavy lock.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/contract-pin-tip-mode/gate-$name"
record="$here/../gate"
mkdir -p "$out" "$record" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
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
cp "$out"/*.exit "$out/head" "$out/clang-version.txt" "$record/"
