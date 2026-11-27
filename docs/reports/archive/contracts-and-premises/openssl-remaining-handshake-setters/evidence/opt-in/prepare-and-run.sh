#!/bin/sh
# Prepare Fluent Bit 046accf6 inputs, build the tip side tree, run the opt-in
# current route on base and tip (sequentially).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a53044d2541350255
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
BIN=$W/private/side-base/target/release/quirin-audit
L=$W/private/logs
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
cd "$W" || exit 2
if [ ! -f "$L/prepare.exit" ] || [ "$(cat "$L/prepare.exit")" != 0 ]; then
  sh "$ENV" "$BIN" prepare --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit --revision 046accf6 \
    --output "$W/private/fb-046accf6" > "$L/prepare.log" 2>&1
  echo $? > "$L/prepare.exit"
fi
mkdir -p "$W/private/side-tip"
git archive 8fffb83ae | tar -x -C "$W/private/side-tip"
echo $? > "$L/side-tip.exit"
sh "$SCRIPT" "$W/private/side-base" "$W/private/opt-in/base-1" "$BIN" "$W/private/fb-046accf6"
sh "$SCRIPT" "$W/private/side-tip" "$W/private/opt-in/tip-1" "$BIN" "$W/private/fb-046accf6"
echo "prepare $(cat "$L/prepare.exit") base $(cat "$W/private/opt-in/base-1/exit") tip $(cat "$W/private/opt-in/tip-1/exit")"
