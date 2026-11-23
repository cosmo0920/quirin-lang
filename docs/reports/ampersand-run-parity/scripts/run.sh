#!/bin/sh
# run.sh NAME TAR -- extract TAR into side-NAME, build its release quirin-audit,
# run the opt-in current route into run-NAME.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ab99aa455e35d7882
N=$1
T=$W/private/arp/side-$N
O=$W/private/arp/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/nullable-lazy-constructor-in-loop/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
