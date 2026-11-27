#!/bin/sh
# sync.sh TAG : providers/contracts.py sync (or check with CHECK=1) through the toolchain
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a43d32eda6d150c67
cd "$W" || exit 2
MODE=sync
[ -n "$CHECK" ] && MODE=check
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh python3 providers/contracts.py $MODE > private/oom/$MODE-$1.log 2>&1
echo $? | tee private/oom/$MODE-$1.exit
tail -5 private/oom/$MODE-$1.log
