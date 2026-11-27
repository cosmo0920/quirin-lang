#!/bin/sh
# run.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into side-NAME, build its release quirin-audit, run the opt-in current route
# into run-NAME. CAST-BEFORE-AMPERSAND-READ-AS-BINARY's run.sh with this
# card's worktree and its verbatim copy of opt-in-current.sh.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a80b57e5e864ae24d
N=$1
T=$W/private/ocod/side-$N
O=$W/private/ocod/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/owned-constructor-out-delivery/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
