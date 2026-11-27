#!/usr/bin/env bash
# integration_check.sh NAME -- the targeted check after a rebase onto a main
# whose new commits touch no file this card touches except the Lean import
# list (the landing-race policy): fmt, clippy, the qcore-c-gateway crate's
# tests, the Lean audits. Every exit to private/cg22/int-NAME/. Run under the
# shared heavy lock.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg22/int-$name"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
step 01-fmt cargo fmt --all -- --check
step 02-clippy cargo clippy --locked --all-targets -- -D warnings
step 03-gateway cargo test --no-fail-fast -p qcore-c-gateway
step 04-lean python3 scripts/check_lean_audits.py
cargo clean --profile dev > "$out/clean.log" 2>&1
