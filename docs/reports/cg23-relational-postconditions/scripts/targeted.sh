#!/bin/sh
# targeted.sh LOG_DIR -- CG23-RELATIONAL-POSTCONDITIONS: the targeted check
# after a rebase onto main commits that are already gated (the landing-race
# policy): cargo check, fmt, clippy, the crates this card and the landed
# cards share, Lean audits. Run under the shared heavy-work flock. Every exit
# is captured to LOG_DIR/<step>.exit; no step is piped.
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
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0
step 01-check cargo check --workspace --all-targets
step 02-fmt cargo fmt --all -- --check
step 03-clippy cargo clippy --locked --all-targets -- -D warnings
step 04-schemas-contract cargo test --no-fail-fast -p qcore-schemas -p qcore-contract
step 05-gateway cargo test --no-fail-fast -p qcore-c-gateway
step 06-lean python3 scripts/check_lean_audits.py
step 07-clean cargo clean --profile dev
