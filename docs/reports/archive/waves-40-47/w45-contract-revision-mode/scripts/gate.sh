#!/usr/bin/env bash
# The standing gate plus the provider gate for W45-CONTRACT-REVISION-MODE, one
# step at a time, each step's output and exit captured to its own file under
# private/rm-gate/ (never piped). Run from the repository root.
set -u
out=private/rm-gate
mkdir -p "$out"
step() {
  name="$1"; shift
  "$@" > "$out/$name.log" 2>&1
  echo $? > "$out/$name.exit"
  echo "$name: $(cat "$out/$name.exit")"
}
step submodules git submodule update --init --recursive
step clang clang --version
step test env CARGO_INCREMENTAL=0 python3 scripts/run_test_binaries.py
step clean cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy env CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
step probe env CARGO_INCREMENTAL=0 cargo run -p qcore-compiler --example reevaluation_probe
step followup env CARGO_INCREMENTAL=0 cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step contracts python3 providers/contracts.py check
step provider-python python3 -m unittest discover -s providers/tests
rm -rf "$out/packages"
step package-build python3 providers/package.py build --output "$out/packages"
for package in qcore.chunkio qcore.cmetrics qcore.ctraces qcore.libevent qcore.librdkafka; do
  step "package-verify-$package" python3 providers/package.py verify "$out/packages/$package"
done
step provider-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
