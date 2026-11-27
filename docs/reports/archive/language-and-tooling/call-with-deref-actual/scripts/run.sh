#!/bin/sh
# run.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into side-NAME, build its release quirin-audit, run the opt-in current route
# into run-NAME. RETURN-FAMILY-UNTRACKED-DEREF-CAST's run.sh with this card's
# worktree and its verbatim copy of opt-in-current.sh.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae8e8b56de03e13f5
N=$1
T=$W/private/cwd/side-$N
O=$W/private/cwd/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
# Base 110a69c8e's providers/Cargo.lock lacks the qcore-c-gateway ->
# qcore-frontend edge MEMBER-EFFECT-STORES added, so the checkers step's
# `--locked` build refuses. Add that one edge in the side tree only (the diff
# is kept beside the run); nothing in the repository changes.
cp providers/Cargo.lock "$O/providers-Cargo.lock.orig"
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo update --offline --workspace --manifest-path providers/Cargo.toml > "$O/lock-refresh.log" 2>&1
diff "$O/providers-Cargo.lock.orig" providers/Cargo.lock > "$O/providers-Cargo.lock.diff"
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/call-with-deref-actual/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
