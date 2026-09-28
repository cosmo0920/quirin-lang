#!/usr/bin/env bash
# W47-FLB-LIB-LIFECYCLE: the standing gate (docs/agents/validation.md)
# plus the provider gate (constraint 18), the pin checks of the Fluent Bit
# crate at the three reviewed revisions, and `pin --verify-contracts` over
# every crate; every exit captured to its own file, logs private.
# usage: gate.sh LOG_DIR rust|records
#   rust:    submodule, Rust suite, clean, fmt, clippy, both probes, Lean
#            audits, clean, then the provider gate and the pin checks
#   records: Python suite and docs-index check (after the matrix and index
#            regeneration), and the Clang recording
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"; phase="$2"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export PATH="$HOME/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
corpus=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
system=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source
case "$phase" in
  rust)
    step 01-submodule git submodule update --init --recursive
    step 02-fmt cargo fmt --all -- --check
    step 03-tests python3 scripts/run_test_binaries.py
    step 04-clean cargo clean --profile dev
    step 05-clippy cargo clippy --all-targets -- -D warnings
    step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
    step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
    step 08-lean python3 scripts/check_lean_audits.py
    step 09-clean cargo clean --profile dev
    step 13-contracts-check python3 providers/contracts.py check
    step 14-package-build python3 providers/package.py build --output "$logs/packages"
    step 15-package-verify python3 providers/package.py verify "$logs/packages/qcore.libevent"
    step 16-provider-python python3 -m unittest discover -s providers/tests
    step 17-provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
    step 18-provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
    step 19-provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
    step 20-provider-clean cargo clean --manifest-path providers/Cargo.toml --profile dev
    step 21-pin-check-66910c10 target/release/quirin-audit pin --source-root "$corpus" --crate quirin-provider-fluent-bit --check
    step 22-pin-check-046accf6 target/release/quirin-audit pin --source-root "$system" --crate quirin-provider-fluent-bit --check
    step 23-pin-check-90978ac6 target/release/quirin-audit pin --source-root "$slice" --crate quirin-provider-fluent-bit --check
    step 24-pin-verify-contracts target/release/quirin-audit pin --verify-contracts
    ;;
  records)
    step 10-python python3 -m unittest discover -s scripts/tests
    step 11-docs-index python3 scripts/build_docs_index.py --check
    step 12-clang /usr/lib/llvm-21/bin/clang --version
    ;;
  *) echo "unknown phase $phase"; exit 2 ;;
esac
