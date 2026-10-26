#!/bin/sh
# MEASURE-TOOL scoped card gate (scripts/gate_scope.py under slots); the
# console log stays outside the worktree.
# Usage: gate.sh WORKTREE LOG_PREFIX OUT_DIR BASE
set -u
cd "$1" || exit 2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
date -u +%Y-%m-%dT%H:%M:%SZ >"$2.start"
git rev-parse HEAD >"$2.head"
python3 scripts/gate_scope.py --base "$4" --out "$3" >"$2.console.log" 2>&1
echo $? >"$2.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$2.end"
