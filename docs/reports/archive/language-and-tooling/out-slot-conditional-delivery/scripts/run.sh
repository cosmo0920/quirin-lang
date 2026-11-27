#!/bin/sh
# run.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into side-NAME, build its release quirin-audit, run the opt-in current route
# into run-NAME. OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY's run.sh with
# OUT-SLOT-CONDITIONAL-DELIVERY's worktree and its verbatim copy of
# opt-in-current.sh.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a246c97e1dc9444ae
N=$1
T=$W/private/oscd/side-$N
O=$W/private/oscd/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/out-slot-conditional-delivery/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
