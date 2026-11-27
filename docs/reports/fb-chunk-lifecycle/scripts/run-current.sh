#!/bin/sh
# run-current.sh SIDE RUN -- the opt-in current route (--monkey-linked) from side-SIDE,
# by RETURN-FAMILY-UNTRACKED-MEMBER-CAST's opt-in-current.sh, verbatim.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aeba8645a8f540281
T=$W/private/fcl/side-$1
O=$W/private/fcl/run-$2
mkdir -p "$O"
sh "$W/docs/reports/fb-chunk-lifecycle/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
