#!/bin/sh
# probe.sh BINARY PROBE_FILE OUT -- run the K2 test binary's zz_probe over one
# body per line (generic d_*/k_* contracts); status and keys per body.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae71e0b9f85fdedfd
cd "$W/crates/qcore-c-gateway" || exit 2
K2BC_PROBE_FILE="$2" sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  "$1" zz_probe --nocapture > "$3" 2>&1
echo "exit $?"
grep -A1 -E 'PROBE|panicked' "$3"
