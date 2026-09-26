#!/usr/bin/env bash
# W45-CONTAINER-ROOT-REACH: the standing gate (docs/agents/validation.md) plus
# the provider gate (constraint 18), every exit captured to its own file, logs
# private. The card adds and changes no contract; the pins are checked at both
# revisions to show it, with this tree's release binary.
# usage: gate.sh LOG_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-container-root-reach/attempt-1/fluent-bit-clang-source
corpus=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
step submodule git submodule update --init --recursive
step tests python3 scripts/run_test_binaries.py
step clean-dev cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step reevaluation-probe cargo run -p qcore-compiler --example reevaluation_probe
step reevaluation-followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step contracts-check python3 providers/contracts.py check
step package-build python3 providers/package.py build --output "$logs/packages"
step package-verify python3 providers/package.py verify "$logs/packages/qcore.libevent"
step provider-python python3 -m unittest discover -s providers/tests
step provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step release-build cargo build --release -p qcore-audit --bin quirin-audit
step pin-check-046accf6 target/release/quirin-audit pin --source-root "$slice" --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit --check
step pin-check-66910c10 target/release/quirin-audit pin --source-root "$corpus" --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit --check
step pin-verify-contracts target/release/quirin-audit pin --verify-contracts --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit
step clang-version /usr/lib/llvm-21/bin/clang --version
