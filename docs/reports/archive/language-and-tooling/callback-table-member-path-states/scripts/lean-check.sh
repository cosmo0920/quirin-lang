#!/bin/sh
# lean-check.sh LOG -- build the card's Lean module and run its audit (axioms).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ac5e43645c6bd6899
cd "$W/proof" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh lake build QCore.CallbackTableMemberPathStates > "$1" 2>&1
echo "build exit $?" >> "$1"
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh lake env lean AuditCallbackTableMemberPathStates.lean >> "$1" 2>&1
echo "audit exit $?" >> "$1"
