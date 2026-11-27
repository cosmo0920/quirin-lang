#!/bin/sh
# Release binaries for the identity series and the arbiter: the branch's
# quirin-audit and Cycle 19 example, and the base commit's quirin-audit built
# from an archive outside the worktree (never a source copy inside it).
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa6cef7bd1d8bab9c
base_rev=${1:-fc6a0f14}
base_tree=${2:?usage: build.sh BASE_REV BASE_TREE_OUTSIDE_WORKTREE}
cd "$root" || exit 2
mkdir -p private/bin private/logs
CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit > private/logs/build-audit.log 2>&1
echo "build quirin-audit exit=$?"
CARGO_INCREMENTAL=0 cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit > private/logs/build-example.log 2>&1
echo "build example exit=$?"
cp target/release/quirin-audit private/bin/quirin-audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit private/bin/example
mkdir -p "$base_tree"
git archive --format=tar -o "$base_tree.tar" "$base_rev"
tar -x -f "$base_tree.tar" -C "$base_tree"
(cd "$base_tree" && CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit) > private/logs/build-base.log 2>&1
echo "build base exit=$?"
cp "$base_tree/target/release/quirin-audit" private/bin/quirin-audit-base
sha256sum private/bin/quirin-audit private/bin/example private/bin/quirin-audit-base
