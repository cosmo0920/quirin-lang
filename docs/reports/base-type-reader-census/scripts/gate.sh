#!/bin/sh
# Card BASE-TYPE-READER-CENSUS: the standing per-change gate, every exit
# captured to its own file and never through a pipe.
#
# Step 5 is an EXTRA step this card adds to its own gate, not part of the
# standing list: every other step runs with `indirect-census` OFF, because no
# workspace member enables it, so that pass lints the instrumentation rather
# than leaving it unlinted.
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a40aa758d95732b25
L="$WT/measure/base-type-reader-census/gate"
mkdir -p "$L"
cd "$WT" || exit 2
: > "$L/exits.txt"
run() {
  name="$1"; shift
  "$@" > "$L/$name.log" 2>&1
  code=$?
  echo "$name=$code" >> "$L/exits.txt"
}
run submodule       git submodule update --init --recursive
run workspace_tests cargo test --workspace --no-fail-fast
run fmt             cargo fmt --all -- --check
run clippy          cargo clippy --all-targets -- -D warnings
run clippy_census   cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings
run probe           cargo run -p qcore-compiler --example reevaluation_probe
run followup        cargo run -p qcore-compiler --example reevaluation_followup
run lean            python3 scripts/check_lean_audits.py
cat "$L/exits.txt"
