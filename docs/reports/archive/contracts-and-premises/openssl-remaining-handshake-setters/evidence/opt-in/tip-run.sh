#!/bin/sh
# Opt-in run of the final implementation tree f1ad773c0 (fresh side tree and path).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a53044d2541350255
BIN=$W/private/side-base/target/release/quirin-audit
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
rm -rf "$W/private/side-tip3"; mkdir -p "$W/private/side-tip3"
cd "$W" || exit 2
git archive f1ad773c0 | tar -x -C "$W/private/side-tip3"
sh "$SCRIPT" "$W/private/side-tip3" "$W/private/opt-in/tip-3" "$BIN" "$W/private/fb-046accf6"
cat "$W/private/opt-in/tip-3/exit"
