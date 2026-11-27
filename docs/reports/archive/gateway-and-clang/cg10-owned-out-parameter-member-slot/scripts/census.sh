#!/bin/sh
# census.sh : gate read census record (qcore-c-gateway) and reduce
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a43d32eda6d150c67
cd "$W" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh python3 scripts/gate_read_census.py record --log-dir private/oom/census-record --packages qcore-c-gateway --jobs 4 > private/oom/census-record.log 2>&1
echo $? > private/oom/census-record.exit
python3 scripts/gate_read_census.py reduce --log-dir private/oom/census-record --out private/oom/census-reduced.json > private/oom/census-reduce.log 2>&1
echo $? > private/oom/census-reduce.exit
