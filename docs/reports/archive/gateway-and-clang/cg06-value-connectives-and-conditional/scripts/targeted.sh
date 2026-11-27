#!/bin/sh
# CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: the targeted check after merging a
# main whose commits were gated on their own (user policy 2026-09-30): fmt,
# clippy, the gateway's tests, Lean audits, python tests, docs index.
# usage: targeted.sh LOG_DIR   (relative to the checkout root)
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$1"
mkdir -p "$logs"
step() {
  name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
}
step 01-submodule git submodule update --init --recursive
step 02-fmt cargo fmt --all -- --check
CARGO_INCREMENTAL=0 step 03-gateway-tests cargo test -p qcore-c-gateway --no-fail-fast
step 05-clippy cargo clippy --all-targets -- -D warnings
step 08-lean python3 scripts/check_lean_audits.py
step 09-python python3 -m unittest discover -s scripts/tests
step 10-index python3 scripts/build_docs_index.py
