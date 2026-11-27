#!/bin/sh
# CG21-CALL-EXITS-AND-BOUNDARIES: gate.sh without its submodule step (run first,
# by hand), for the shared heavy lock. From gate.sh: the
# standing gate, plus the provider check this card owes (it changes
# providers/quirin-provider-libc). usage: gate.sh LOG_DIR (relative to the
# checkout root). Every exit is captured to LOG_DIR/<step>.exit; no step is
# piped. `cargo clean --profile dev` follows the test runner (shared host).
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
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
step 02-fmt cargo fmt --all -- --check
step 03-tests python3 scripts/run_test_binaries.py
step 04-clean cargo clean --profile dev
step 05-clippy cargo clippy --locked --all-targets -- -D warnings
step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
step 08-lean python3 scripts/check_lean_audits.py
clang --version > "$logs/clang-version.txt" 2>&1
echo $? > "$logs/clang-version.exit"
step 09-python python3 -m unittest discover -s scripts/tests
step 10-index python3 scripts/build_docs_index.py --check
step 11-providers-check cargo check --manifest-path providers/Cargo.toml --workspace --locked --offline
step 12-providers-libc-test cargo test --manifest-path providers/Cargo.toml -p quirin-provider-libc --locked --offline
step 13-clean cargo clean --profile dev
