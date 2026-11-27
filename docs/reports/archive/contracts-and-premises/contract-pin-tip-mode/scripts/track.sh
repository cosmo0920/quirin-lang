#!/bin/bash
# Record mk_vhost_set's tip entry (reads the Monkey repository; writes only the libevent lock).
W=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-abb94a55c5f4a07cd
cd "$W"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cargo run -q -p qcore-audit --bin quirin-audit -- pin \
  --source-root /home/cosmo/Documents/GitHub/monkey \
  --crate qcore-provider-libevent --function mk_vhost_set \
  --track-tip cosmo0920-missing-va_end-on-null \
  --repository-name monkey --repository-url https://github.com/cosmo0920/monkey \
  --release-label "Monkey release after the va_end fix (release candidate)" \
  > "$W/private/contract-pin-tip-mode/track-tip.log" 2>&1
echo $? > "$W/private/contract-pin-tip-mode/track-tip.exit"
