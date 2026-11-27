#!/bin/sh
# build.sh SIDE -- build the release quirin-audit in private/fcl/side-SIDE
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aeba8645a8f540281
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
cd "$W/private/fcl/side-$1" || exit 2
sh "$ENV" cargo build --release -p qcore-audit --bin quirin-audit > "$W/private/fcl/$1-build.log" 2>&1
echo $? > "$W/private/fcl/$1-build.exit"
sha256sum target/release/quirin-audit > "$W/private/fcl/$1-binary.sha256"
