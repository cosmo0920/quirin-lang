#!/bin/sh
# gate.sh OUT: the scoped card gate through the private toolchain env.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a4697725f5e3891df
cd "$W" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  python3 scripts/gate_scope.py --base 0f6e17118 --out "$1" > "$1.log" 2>&1
e=$?
echo $e > "$1.exit"
echo "exit $e"
exit $e
