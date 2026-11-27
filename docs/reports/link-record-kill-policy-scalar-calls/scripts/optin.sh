#!/bin/sh
# optin.sh NAME -- the opt-in current --monkey-linked route from
# private/m/side-NAME's own tree and release binary into private/m/optin-NAME.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a886acca29e222359
N=$1
T=$W/private/m/side-$N
sh $W/private/scripts/opt-in-current.sh "$T" $W/private/m/optin-$N "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
