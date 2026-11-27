#!/bin/sh
# Standing gate plus provider gate, every exit captured to a file, nothing
# piped. usage: gate.sh RUN  (logs in the worktree-private measure/RUN)
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa6cef7bd1d8bab9c
cd "$root" || exit 2
out="$root/measure/${1:?usage: gate.sh RUN}"
if [ -e "$out" ]; then
  echo "$out exists" >&2
  exit 2
fi
mkdir -p "$out"
export CARGO_INCREMENTAL=0
export PATH="$HOME/.elan/bin:$PATH"
step() {
  name=$1
  shift
  uptime > "$out/$name.load"
  "$@" > "$out/$name.log" 2>&1
  status=$?
  echo "$status" > "$out/$name.exit"
  echo "$name exit=$status"
}
step submodules git submodule update --init --recursive
step clang clang --version
step test python3 scripts/run_test_binaries.py
step clean cargo clean --profile dev
step fmt cargo fmt --all -- --check
step clippy cargo clippy --all-targets -- -D warnings
step clippy-workspace cargo clippy --workspace --all-targets -- -D warnings
step probe cargo run -p qcore-compiler --example reevaluation_probe
step followup cargo run -p qcore-compiler --example reevaluation_followup
step lean python3 scripts/check_lean_audits.py --jobs 4
step python python3 -m unittest discover -s scripts/tests
step build-offline cargo build --offline --workspace
step contracts-check python3 providers/contracts.py check
step provider-python python3 -m unittest discover -s providers/tests
rm -rf "$out/packages"
step package-build python3 providers/package.py build --output "$out/packages"
for package in "$out"/packages/*/; do
  name=$(basename "$package")
  step "package-verify-$name" python3 providers/package.py verify "$package"
done
step provider-test cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast
step provider-mutation-isolated cargo test --manifest-path providers/Cargo.toml -p qcore-provider-conformance --test packages --locked --offline -- --exact real_packages_preserve_unknown_and_reject_mutations --test-threads=1
step provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
echo "gate done"
