#!/bin/bash
# pin --check of the libevent crate against the Monkey checkout (read only).
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-abb94a55c5f4a07cd
P=$W/private/contract-pin-tip-mode
cd "$W"
B=$W/target/debug/quirin-audit
"$B" pin --source-root /home/cosmo/Documents/GitHub/monkey --crate qcore-provider-libevent \
  --function mk_vhost_set --check > "$P/monkey-check.log" 2>&1
echo $? > "$P/monkey-check.exit"
"$B" pin --source-root /home/cosmo/Documents/GitHub/monkey --crate qcore-provider-libevent \
  --check > "$P/monkey-check-all.log" 2>&1
echo $? > "$P/monkey-check-all.exit"
"$B" pin --verify-contracts > "$P/verify-contracts.log" 2>&1
echo $? > "$P/verify-contracts.exit"
