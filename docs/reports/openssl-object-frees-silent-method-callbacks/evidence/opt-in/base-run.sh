#!/bin/sh
# Opt-in run of the base tree ecc03d7a4 (integrate/r39; fresh side tree),
# with the base's release quirin-audit (the card changes no crate source).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a30378b5b0886fdc8
BIN=$W/target/release/quirin-audit
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
rm -rf "$W/private/side-base"; mkdir -p "$W/private/side-base" "$W/private/opt-in"
cd "$W" || exit 2
git archive ecc03d7a4 | tar -x -C "$W/private/side-base"
sh "$SCRIPT" "$W/private/side-base" "$W/private/opt-in/base-1" "$BIN" "$W/private/fb-046accf6"
echo "base $(cat "$W/private/opt-in/base-1/exit")" > "$W/private/logs/base-run.done"
