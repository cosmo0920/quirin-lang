#!/usr/bin/env bash
# gate.sh {standing-pre|standing-tests|providers-pre|providers-tests|docs}; coordinate heavy phases.
set -u
part="$1"
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
case "$part" in standing-*) group=standing;; providers-*) group=providers;; *) group="$part";; esac
out="$repo/private/rk-interceptor-candidates/gate-$group${2:-}"
mkdir -p "$out"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/rk-interceptor-candidates-target
failed=0
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  local code=$?
  echo "$code" > "$out/$label.exit"
  echo "$label $code"
  if [ "$code" != 0 ]; then failed=1; fi
}
git rev-parse HEAD > "$out/head-$part"
case "$part" in
  standing-pre)
    step fmt cargo fmt --all -- --check
    step clippy cargo clippy --all-targets -- -D warnings
    step probe cargo run -p qcore-compiler --example reevaluation_probe
    step followup cargo run -p qcore-compiler --example reevaluation_followup
    step lean python3 scripts/check_lean_audits.py
    clang --version > "$out/clang-version.txt" 2>&1
    echo $? > "$out/clang-version.exit"
    ;;
  standing-tests)
    step tests python3 scripts/run_test_binaries.py --log-dir "$out/test-binaries"
    ;;
  providers-pre)
    step check cargo check --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline
    step fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
    step clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
    step contracts python3 providers/contracts.py check
    step package-build python3 providers/package.py build --output "$out/packages" --target-dir "$CARGO_TARGET_DIR/providers-packages"
    for package in "$out/packages"/*; do
      [ -d "$package" ] || continue
      step "verify-$(basename "$package")" python3 providers/package.py verify "$package"
    done
    ;;
  providers-tests)
    step tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
    step python python3 -m unittest discover -s providers/tests
    ;;
  docs)
    step python python3 -m unittest discover -s scripts/tests
    step index python3 scripts/build_docs_index.py --check
    ;;
  *) exit 2;;
esac
exit "$failed"
