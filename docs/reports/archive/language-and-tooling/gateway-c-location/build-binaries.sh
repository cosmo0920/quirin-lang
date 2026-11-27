#!/bin/sh
# Build the BEFORE and AFTER audit binaries in this worktree (card
# GATEWAY-C-LOCATION, wave 27).
#
# Both are built here, from the same manifest directory, with the base sources
# checked out over the tip's and restored afterwards — the wave-25 route
# detail: a `git archive` of the base has no submodule, so the libevent pin
# gate reads the enclosing repository's HEAD and the run is void.
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a1e06161086be80b0
BASE=f122cc03
PATHS="crates compiler proof"
cd "$WT"
mkdir -p measure/bin

cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit measure/bin/after

git checkout "$BASE" -- $PATHS
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit measure/bin/before
git checkout HEAD -- $PATHS
git status --short

sha256sum measure/bin/before measure/bin/after
echo BUILD_OK
