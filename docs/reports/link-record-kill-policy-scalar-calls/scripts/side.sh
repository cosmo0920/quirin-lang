#!/bin/sh
# side.sh NAME -- build private/m/side-NAME's release quirin-audit, then run
# the fluent-bit-system route and the r83 census units from it.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a886acca29e222359
N=$1
T=$W/private/m/side-$N
S=$W/docs/reports/leri-held-exit-class-precondition/scripts
mkdir -p $W/private/m/sys-$N $W/private/m/census-$N
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > $W/private/m/sys-$N/build.log 2>&1
echo $? > $W/private/m/sys-$N/build.exit
sha256sum "$T/target/release/quirin-audit" > $W/private/m/sys-$N/binary.sha256
sh $S/system.sh "$T" $W/private/m/sys-$N
sh $S/census-units.sh "$T" $W/private/m/census-$N
