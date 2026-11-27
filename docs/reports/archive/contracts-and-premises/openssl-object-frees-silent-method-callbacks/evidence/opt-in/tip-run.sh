#!/bin/sh
# Opt-in run of the implementation tree b80ee7664 (fresh side tree), with the
# base's release quirin-audit (the card changes no crate source).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a30378b5b0886fdc8
BIN=$W/target/release/quirin-audit
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
rm -rf "$W/private/side-tip"; mkdir -p "$W/private/side-tip" "$W/private/opt-in"
cd "$W" || exit 2
git archive b80ee7664 | tar -x -C "$W/private/side-tip"
sh "$SCRIPT" "$W/private/side-tip" "$W/private/opt-in/tip-1" "$BIN" "$W/private/fb-046accf6"
echo "tip $(cat "$W/private/opt-in/tip-1/exit")" > "$W/private/logs/tip-run.done"
