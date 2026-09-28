#!/usr/bin/env bash
# fixture_before_after.sh TREE OUT -- run fixture_before_after.rs inside TREE
# (the base export for BEFORE, this worktree for AFTER) and write its ROW
# lines to OUT; the probe file is removed from TREE afterwards. Exit captured.
set -u
tree="$(cd "$1" && pwd -P)"
out="$2"
here="$(cd "$(dirname "$0")" && pwd)"
probe="$tree/crates/qcore-c-gateway/tests/zz_fixture_before_after.rs"
cp "$here/fixture_before_after.rs" "$probe" || exit 2
cd "$tree" || exit 2
CARGO_INCREMENTAL=0 cargo test -p qcore-c-gateway --test zz_fixture_before_after -- --nocapture \
  > "$out.log" 2>&1
echo $? > "$out.exit"
rm -f "$probe"
grep '^ROW ' "$out.log" > "$out"
