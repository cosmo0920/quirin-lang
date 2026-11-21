#!/bin/sh
# build.sh TAG [census] : release build of quirin-audit in this worktree, copied to private/r4/bin/quirin-audit-TAG
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aab78fbbfefedf82b
cd "$W" || exit 2
mkdir -p private/r4/bin
F=""
[ "$2" = census ] && F="--features qcore-c-gateway/indirect-census"
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit $F > private/r4/build-$1.log 2>&1
echo $? > private/r4/build-$1.exit
cp target/release/quirin-audit private/r4/bin/quirin-audit-$1
sha256sum private/r4/bin/quirin-audit-$1
