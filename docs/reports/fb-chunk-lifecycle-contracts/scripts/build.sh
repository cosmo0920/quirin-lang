#!/bin/sh
# build.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into private/fclc/side-NAME and build its release quirin-audit.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a273dcc45e6a8fab7
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
T=$W/private/fclc/side-$1
mkdir -p "$T"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh "$ENV" cargo build --release -p qcore-audit --bin quirin-audit > "$W/private/fclc/$1-build.log" 2>&1
echo $? > "$W/private/fclc/$1-build.exit"
sha256sum target/release/quirin-audit > "$W/private/fclc/$1-binary.sha256"
