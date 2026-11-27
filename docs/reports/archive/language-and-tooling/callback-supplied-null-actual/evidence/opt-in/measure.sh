#!/bin/sh
# CALLBACK-SUPPLIED-NULL-ACTUAL opt-in measurement driver.
# sh private/measure.sh NAME REV [experiment]
set -u
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a189b2c75b21c6ea5
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
NAME=$1
REV=$2
SIDE=$W/private/side/$NAME
LOG=$W/private/logs/measure-$NAME.log
exec > "$LOG" 2>&1
if [ ! -x "$SIDE/target/release/quirin-audit" ]; then
  mkdir -p "$SIDE"
  git -C "$W" archive "$REV" | tar -x -C "$SIDE"
  cp -r "$W/third_party/libevent/." "$SIDE/third_party/libevent/"
  rm -rf "$SIDE/third_party/libevent/.git"
  if [ "${3:-}" = experiment ]; then
    python3 "$W/private/experiment_patch.py" "$SIDE/crates/qcore-c-gateway/src/callback_contracts.rs" || exit 3
  fi
  sh "$ENV" cargo build --release --manifest-path "$SIDE/Cargo.toml" -p qcore-audit --bin quirin-audit || exit 4
fi
sh "$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh" \
  "$SIDE" "$W/private/opt-in/$NAME-1" "$SIDE/target/release/quirin-audit" "$W/private/fb-046accf6"
echo "measure $NAME done exit $(cat "$W/private/opt-in/$NAME-1/exit")"
