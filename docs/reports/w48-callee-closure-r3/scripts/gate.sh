#!/usr/bin/env bash
# W48-CALLEE-CLOSURE-R3: the standing gate and the provider gate
# (docs/agents/validation.md; adapted from w47-callee-closure-r2/scripts/gate.sh).
# Every exit is captured to its own file, no gate step is piped, logs are private.
# usage: gate.sh LOG_DIR PACKAGE_DIR
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"
packages="$2"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=6
export PATH="$HOME/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")" >> "$logs/steps.txt"
}
git rev-parse HEAD > "$logs/head.txt"
step 01-submodule git submodule update --init --recursive
step 03-tests python3 scripts/run_test_binaries.py
step 04-clean-dev cargo clean --profile dev
step 02-fmt cargo fmt --all -- --check
step 05-clippy cargo clippy --all-targets -- -D warnings
step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
step 08-lean python3 -B scripts/check_lean_audits.py
step 09-python python3 -m unittest discover -s scripts/tests
step 10-index python3 scripts/build_docs_index.py --check
step 11-provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step 12-provider-clean-dev cargo clean --manifest-path providers/Cargo.toml --profile dev
step 13-provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step 14-provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step 15-provider-contracts-check python3 providers/contracts.py check
step 16-package-build python3 providers/package.py build --output "$packages"
for package in qcore.chunkio qcore.cmetrics qcore.ctraces qcore.libevent qcore.librdkafka; do
  step "17-package-verify-$package" python3 providers/package.py verify "$packages/$package"
done
step 18-provider-python python3 -m unittest discover -s providers/tests
step 19-provider-clean cargo clean --manifest-path providers/Cargo.toml --profile dev
step 20-clean-dev cargo clean --profile dev
clang --version > "$logs/clang-version.txt" 2>&1
echo $? > "$logs/clang-version.exit"
echo done > "$logs/ALL_DONE"
