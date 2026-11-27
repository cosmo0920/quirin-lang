#!/bin/sh
# Opt-in run of the implementation tree c282a01f7 (fresh side tree), with the
# base binary (the card changes no crate source outside tests).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aaf5d432a4505fc36
BIN=$W/private/side-base/target/release/quirin-audit
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
rm -rf "$W/private/side-tip"; mkdir -p "$W/private/side-tip"
cd "$W" || exit 2
git archive c282a01f7 | tar -x -C "$W/private/side-tip"
sh "$SCRIPT" "$W/private/side-tip" "$W/private/opt-in/tip-1" "$BIN" "$W/private/fb-046accf6"
echo "tip $(cat "$W/private/opt-in/tip-1/exit")" > "$W/private/logs/tip-run.done"
