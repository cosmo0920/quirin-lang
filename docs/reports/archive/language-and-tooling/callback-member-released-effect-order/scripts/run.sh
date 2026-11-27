#!/bin/sh
# run.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into side-NAME, build its release quirin-audit, run the opt-in current route
# into run-NAME. CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD's run.sh with this
# card's worktree and its verbatim copy of opt-in-current.sh.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a08c5916e3a82bcdf
N=$1
T=$W/private/cmeo/side-$N
O=$W/private/cmeo/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/callback-member-released-effect-order/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
