#!/usr/bin/env bash
# gate.sh {standing|providers|docs}; run only in a coordinated heavy slot.
# Every command records its own exit and complete output under private/.
set -u
part="$1"
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ "$part" = docs ]; then
  out="$repo/private/rk-outer/gate-docs"
else
  out="$repo/private/rk-outer/gate-$part-v2"
fi
mkdir -p "$out"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/rk-outer-target
failed=0
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  local code=$?
  echo "$code" > "$out/$label.exit"
  echo "$label $code"
  if [ "$code" != 0 ]; then failed=1; fi
}
git rev-parse HEAD > "$out/head"
case "$part" in
  standing)
    step submodule git submodule update --init --recursive
    step tests python3 scripts/run_test_binaries.py --log-dir "$out/test-binaries"
    step fmt cargo fmt --all -- --check
    step clippy cargo clippy --all-targets -- -D warnings
    step probe cargo run -p qcore-compiler --example reevaluation_probe
    step followup cargo run -p qcore-compiler --example reevaluation_followup
    step lean python3 scripts/check_lean_audits.py
    clang --version > "$out/clang-version.txt" 2>&1
    code=$?
    echo "$code" > "$out/clang-version.exit"
    if [ "$code" != 0 ]; then failed=1; fi
    ;;
  providers)
    step check cargo check --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline
    step tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
    step fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
    step clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
    step contracts python3 providers/contracts.py check
    step package-build python3 providers/package.py build --output "$out/packages"
    for package in "$out/packages"/*; do
      [ -d "$package" ] || continue
      step "verify-$(basename "$package")" python3 providers/package.py verify "$package"
    done
    step python python3 -m unittest discover -s providers/tests
    ;;
  docs)
    step python python3 -m unittest discover -s scripts/tests
    step index python3 scripts/build_docs_index.py --check
    ;;
  *) echo "unknown part: $part" >&2; exit 2;;
esac
exit "$failed"
