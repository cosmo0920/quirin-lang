#!/bin/sh
# run.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into side-NAME, build its release quirin-audit, run the opt-in current route
# into run-NAME. CG25-IN-PLACE-REGEX's run.sh with this card's
# worktree and its verbatim copy of opt-in-current.sh (copied from
# FRAME-HOLDER-SHADOWED-NAME with only the paths changed).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aa20cfec639310cba
N=$1
T=$W/private/nkh/side-$N
O=$W/private/nkh/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/name-keyed-holder-shadowing/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
