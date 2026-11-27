#!/usr/bin/env bash
# W47-REVIEW-HOLD-CONCRETE-REASON: the standing gate (docs/agents/validation.md)
# and the provider gate (providers/ changed: two contract modules and the
# Fluent Bit lock). Every exit is captured to its own file; nothing is piped;
# logs are private. The submodule step is run by hand first, its exit recorded
# as 01-submodule.exit in LOG_DIR.
# usage: gate.sh LOG_DIR rust|provider|records
#   rust:     fmt, Rust suite, clean, clippy, both probes, Lean audits, clean
#   provider: provider cargo test, clean, fmt, clippy, contracts check,
#             package build and five verifies, provider Python
#   records:  Python suite and docs-index check (after the matrix and index
#             regeneration), and the Clang recording
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
logs="$1"; phase="$2"
mkdir -p "$logs"
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
export PATH="/home/cosmo/.elan/bin:$PATH"
provider=(--manifest-path providers/Cargo.toml)
packages="$logs/packages"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
case "$phase" in
  rust)
    step 02-fmt cargo fmt --all -- --check
    step 03-tests python3 scripts/run_test_binaries.py
    step 04-clean cargo clean --profile dev
    step 05-clippy cargo clippy --all-targets -- -D warnings
    step 06-probe cargo run -p qcore-compiler --example reevaluation_probe
    step 07-followup cargo run -p qcore-compiler --example reevaluation_followup
    step 08-lean python3 scripts/check_lean_audits.py
    step 09-clean cargo clean --profile dev
    ;;
  provider)
    step 13-provider-tests cargo test "${provider[@]}" --workspace --locked --offline
    step 14-provider-clean cargo clean "${provider[@]}" --profile dev
    step 15-provider-fmt cargo fmt "${provider[@]}" --all -- --check
    step 16-provider-clippy cargo clippy "${provider[@]}" --workspace --all-targets --locked \
      --offline -- -D warnings
    step 17-contracts-check python3 providers/contracts.py check
    step 18-package-build python3 providers/package.py build --output "$packages"
    for package in chunkio cmetrics ctraces libevent librdkafka; do
      step "19-verify-$package" python3 providers/package.py verify "$packages/qcore.$package"
    done
    step 20-provider-python python3 -m unittest discover -s providers/tests
    step 21-provider-clean cargo clean "${provider[@]}" --profile dev
    ;;
  records)
    step 10-python python3 -m unittest discover -s scripts/tests
    step 11-docs-index python3 scripts/build_docs_index.py --check
    step 12-clang /usr/lib/llvm-21/bin/clang --version
    ;;
  *) echo "unknown phase $phase"; exit 2 ;;
esac
