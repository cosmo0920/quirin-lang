#!/usr/bin/env bash
# gate.sh NAME [PART] -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES (from PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS's, copied from
# PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE's, from HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's, from EXECUTION-QUOTA-LOAD-INDEPENDENCE's scripts/gate.sh): cargo check and the
# standing per-change gate (docs/agents/validation.md), every step's output and
# exit to private/opctor/gate-NAME/, nothing piped, each step under the shared
# heavy lock. PART is `code` (steps 00-07, plus the provider gate as 10-15,
# because providers/ changed) or `docs` (08-09, after the
# records) or `all`.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=$(ls -d /tmp/claude-1000/*/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock 2>/dev/null | head -1)
: "${lock:=$repo/private/opctor/heavy.lock}"
private="$repo/private/opctor"
out="$private/gate-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  # HELD=1: the caller already holds the heavy lock for the whole gate
  # (`flock <lock> bash gate-whole.sh`), so a step must not take it
  # again (a second flock on the same file would wait for the caller forever).
  local label="$1"; shift
  if [ "${HELD:-0}" = 1 ]; then
    "$@" > "$out/$label.log" 2>&1
  else
    flock "$lock" "$@" > "$out/$label.log" 2>&1
  fi
  echo $? > "$out/$label.exit"
}
if [ "$part" != docs ]; then
  git rev-parse HEAD > "$out/head"
  step 00-check cargo check --workspace --all-targets
  step 01-submodule git submodule update --init --recursive
  step 02-fmt cargo fmt --all -- --check
  step 03-clippy cargo clippy --locked --all-targets -- -D warnings
  step 04-tests python3 scripts/run_test_binaries.py --log-dir "$private/tests-$name"
  step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
  step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
  step 07-lean python3 scripts/check_lean_audits.py
  clang --version > "$out/clang-version.txt" 2>&1
  echo $? > "$out/clang-version.exit"
  rm -rf "$private/packages-$name"
  step 10-packages-build python3 providers/package.py build --output "$private/packages-$name"
  step 11-packages-verify python3 providers/package.py verify "$private/packages-$name/qcore.libevent"
  step 11a-packages-verify-librdkafka python3 providers/package.py verify "$private/packages-$name/qcore.librdkafka"
  step 11b-provider-check cargo check --manifest-path providers/Cargo.toml --workspace --locked --offline
  step 11c-contracts-check python3 providers/contracts.py check
  step 12-provider-tests cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
  step 13-provider-python python3 -m unittest discover -s providers/tests
  step 14-provider-fmt cargo fmt --manifest-path providers/Cargo.toml --all -- --check
  step 15-provider-clippy cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
  echo done > "$out/code-done"
fi
if [ "$part" != code ]; then
  git rev-parse HEAD > "$out/docs-head"
  step 08-python python3 -m unittest discover -s scripts/tests
  step 09-index python3 scripts/build_docs_index.py --check
  echo done > "$out/docs-done"
fi
