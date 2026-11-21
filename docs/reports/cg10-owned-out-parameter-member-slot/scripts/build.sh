#!/bin/sh
# build.sh TAG : release build of quirin-audit in this worktree, copied to private/oom/bin/quirin-audit-TAG
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a43d32eda6d150c67
cd "$W" || exit 2
mkdir -p private/oom/bin
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > private/oom/build-$1.log 2>&1
echo $? > private/oom/build-$1.exit
cp target/release/quirin-audit private/oom/bin/quirin-audit-$1
sha256sum private/oom/bin/quirin-audit-$1
cat private/oom/build-$1.exit
