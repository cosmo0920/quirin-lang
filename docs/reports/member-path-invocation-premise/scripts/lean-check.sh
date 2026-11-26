#!/bin/sh
# lean-check.sh LOG -- build the card's Lean module and run its audit (axioms).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ad28b60a7514cefed
cd "$W/proof" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh lake build QCore.MemberPathInvocationPremise > "$1" 2>&1
echo "build exit $?" >> "$1"
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh lake env lean AuditMemberPathInvocationPremise.lean >> "$1" 2>&1
echo "audit exit $?" >> "$1"
