#!/bin/bash
# stage.sh <stage-name>
set -u
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a169a5e9421edcb6d
M=$WT/measure/scalar-route-exit-edge
STAGE=$1
cd "$WT" || exit 90
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit > "$M/build-$STAGE.log" 2>&1
BUILD=$?
echo "BUILD_EXIT=$BUILD" > "$M/exit-$STAGE.txt"
[ $BUILD -ne 0 ] && exit 91
cp target/release/examples/cycle19_fluent_bit_libraries_audit "$M/bin/$STAGE" || exit 92
python3 -B "$M/run_audit.py" "$STAGE" "$M/bin/$STAGE" > "$M/audit-$STAGE.log" 2>&1
echo "AUDIT_EXIT=$?" >> "$M/exit-$STAGE.txt"
