#!/bin/sh
# gate.sh OUT: the scoped card gate (docs/agents/validation.md), through the
# host toolchain (system Clang 21, Lean 4.35). Exit to OUT.exit.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a717a6f321d7ff5d9
cd "$W" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  python3 scripts/gate_scope.py --base d7a497822 --out "$1" > "$1.log" 2>&1
echo $? > "$1.exit"
