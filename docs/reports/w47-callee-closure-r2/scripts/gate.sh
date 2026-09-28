#!/usr/bin/env bash
# W47-CALLEE-CLOSURE-R2 standing gate plus the provider gate
# (docs/agents/validation.md; adapted from w45-sr5-soundness/scripts/gate.sh).
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
export PATH="$HOME/.elan/bin:$PATH"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")" >> "$logs/steps.txt"
}
git rev-parse HEAD > "$logs/head.txt"
step submodule git submodule update --init --recursive
step tests python3 scripts/run_test_binaries.py
step clean-dev cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step reevaluation-probe cargo run -p qcore-compiler --example reevaluation_probe
step reevaluation-followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 -B scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
step provider-clean-dev cargo clean --manifest-path providers/Cargo.toml --profile dev
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
step provider-contracts-check python3 providers/contracts.py check
step package-build python3 providers/package.py build --output "$packages"
for package in qcore.chunkio qcore.cmetrics qcore.ctraces qcore.libevent qcore.librdkafka; do
  step "package-verify-$package" python3 providers/package.py verify "$packages/$package"
done
step provider-python python3 -m unittest discover -s providers/tests
step clang-version clang --version
echo done > "$logs/ALL_DONE"
