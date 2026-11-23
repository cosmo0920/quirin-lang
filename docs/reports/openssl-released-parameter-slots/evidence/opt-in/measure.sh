#!/bin/sh
# OPENSSL-RELEASED-PARAMETER-SLOTS opt-in measurement driver.
# sh private/measure.sh NAME REV [prepare]
set -u
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a52d6afd31bcc2d7d
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
NAME=$1
REV=$2
SIDE=$W/private/side/$NAME
LOG=$W/private/logs/measure-$NAME.log
mkdir -p "$W/private/logs"
exec > "$LOG" 2>&1
if [ ! -x "$SIDE/target/release/quirin-audit" ]; then
  mkdir -p "$SIDE"
  git -C "$W" archive "$REV" | tar -x -C "$SIDE"
  cp -r "$W/third_party/libevent/." "$SIDE/third_party/libevent/"
  rm -rf "$SIDE/third_party/libevent/.git"
  sh "$ENV" cargo build --release --manifest-path "$SIDE/Cargo.toml" -p qcore-audit --bin quirin-audit || exit 4
fi
if [ "${3:-}" = prepare ] && [ ! -f "$W/private/fb-046accf6/prepare.json" ]; then
  sh "$ENV" "$SIDE/target/release/quirin-audit" prepare --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
    --revision 046accf6 --output "$W/private/fb-046accf6" > "$W/private/logs/prepare.log" 2>&1
  echo "prepare exit $?"
fi
sh "$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh" \
  "$SIDE" "$W/private/opt-in/$NAME-1" "$SIDE/target/release/quirin-audit" "$W/private/fb-046accf6"
echo "measure $NAME done exit $(cat "$W/private/opt-in/$NAME-1/exit")"
