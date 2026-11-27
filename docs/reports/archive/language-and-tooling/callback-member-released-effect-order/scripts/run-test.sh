#!/bin/bash
# run-test.sh LOG TARGET [FILTER...] -- one gateway test target under the test link config.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a08c5916e3a82bcdf
cd "$W" || exit 2
LOG=$1; shift
T=$1; shift
mapfile -t tc < <(python3 scripts/test_link_config.py)
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo test "${tc[@]}" -p qcore-c-gateway --test "$T" -- --nocapture "$@" > "$LOG" 2>&1
echo $? > "$LOG.exit"
