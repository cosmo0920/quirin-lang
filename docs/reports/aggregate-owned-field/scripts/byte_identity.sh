#!/usr/bin/env bash
# Byte-identity census for card AGGREGATE-OWNED-FIELD (wave 39).
#
# Builds the census tool (custody_census.rs, beside this script) in two trees
# — the base commit's scratch extraction and the card's own tree — runs each
# FROM ITS OWN ROOT over the same relative spelled paths (standing constraint
# 11: the spelled input path reaches `__FILE__` and the `#line` directive, so a
# differing prefix would be a false difference), and compares the outputs per
# program: profile, custody operation list, emitted C and reference observation.
#
# Usage:  byte_identity.sh <base tree> <card tree> <out dir>
# The out dir receives base.txt, card.txt and summary.txt; every exit code is
# written to a file rather than piped.
set -u
base="$1"; card="$2"; out="$3"
mkdir -p "$out"
here="$(cd "$(dirname "$0")" && pwd)"
roots="compiler/tests/fixtures examples crates contract docs/reports"

for tree in "$base" "$card"; do
  cp "$here/custody_census.rs" "$tree/compiler/examples/custody_census.rs"
done

( cd "$base" && cargo build --release -q --example custody_census -p qcore-compiler ) \
  > "$out/build-base.log" 2>&1; echo $? > "$out/build-base.exit"
( cd "$card" && cargo build --release -q --example custody_census -p qcore-compiler ) \
  > "$out/build-card.log" 2>&1; echo $? > "$out/build-card.exit"

( cd "$base" && ./target/release/examples/custody_census $roots ) \
  > "$out/base.txt" 2> "$out/base.err"; echo $? > "$out/run-base.exit"
( cd "$card" && ./target/release/examples/custody_census $roots ) \
  > "$out/card.txt" 2> "$out/card.err"; echo $? > "$out/run-card.exit"

# The card tree's copy of the tool is measurement scaffolding, not a shipped
# example: remove it so the tree stays what was committed.
rm -f "$card/compiler/examples/custody_census.rs"

python3 "$here/compare_census.py" "$out/base.txt" "$out/card.txt" > "$out/summary.txt" 2>&1
echo $? > "$out/compare.exit"
cat "$out/summary.txt"
