#!/bin/bash
# census.sh — BEFORE-behaviour build with the polarity/arm census, then restore.
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a169a5e9421edcb6d
M=$WT/measure/scalar-route-exit-edge
SRC=$WT/crates/qcore-c-gateway/src/legacy_c.rs
cd "$WT" || exit 90
cp "$SRC" "$M/legacy_c.rs.pristine" || exit 91
python3 -B "$M/unrestrict.py" "$M/legacy_c.rs.pristine" "$M/legacy_c.rs.unrestricted" > "$M/census-patch.log" 2>&1 || { cp "$M/legacy_c.rs.pristine" "$SRC"; exit 92; }
python3 -B "$M/instrument_census.py" "$M/legacy_c.rs.unrestricted" "$M/legacy_c.rs.census" >> "$M/census-patch.log" 2>&1 || { cp "$M/legacy_c.rs.pristine" "$SRC"; exit 93; }
cp "$M/legacy_c.rs.census" "$SRC"
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit > "$M/build-census.log" 2>&1
BUILD=$?
echo "BUILD_EXIT=$BUILD" > "$M/exit-census.txt"
if [ $BUILD -eq 0 ]; then
  cp target/release/examples/cycle19_fluent_bit_libraries_audit "$M/bin/census"
  rm -f "$M/census.jsonl"
  QCORE_SRE_CENSUS="$M/census.jsonl" python3 -B "$M/run_audit.py" census "$M/bin/census" > "$M/audit-census.log" 2>&1
  echo "AUDIT_EXIT=$?" >> "$M/exit-census.txt"
fi
# restore, mtime advanced
cp "$M/legacy_c.rs.pristine" "$SRC"
touch "$SRC"
echo "RESTORED" >> "$M/exit-census.txt"
