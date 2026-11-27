#!/bin/sh
# build-src.sh DIR NAME: release-build quirin-audit + measure plugin in DIR
D=$1; N=$2; L=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a5d0fa65ce9b509f2/private/logs
cd "$D" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit -p quirin-audit-plugin-measure -j 12 > "$L/$N-build.log" 2>&1
e=$?; echo $e > "$L/$N-build.exit"; exit $e
