#!/usr/bin/env bash
# gate.sh RUN -- full standing gate into one fresh worktree-private directory.
# The coordinator grants the heavy-run slot before this script starts.
set -u
run="${1:?fresh run name}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/w48-pointer-return/gate-$run"
if [ -e "$out" ]; then
  echo "gate directory already exists: $out" >&2
  exit 2
fi
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"
  shift
  "$@" > "$out/$label.log" 2>&1
  echo "$?" > "$out/$label.exit"
}
step 00-submodule git submodule update --init --recursive
step 01-tests python3 scripts/run_test_binaries.py --log-dir "$out/test-binaries"
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --all-targets -- -D warnings
step 04-probe cargo run -p qcore-compiler --example reevaluation_probe
step 05-followup cargo run -p qcore-compiler --example reevaluation_followup
step 06-lean python3 scripts/check_lean_audits.py
step 07-python python3 -m unittest discover -s scripts/tests
step 08-index python3 scripts/build_docs_index.py --check
step 09-clang clang --version
