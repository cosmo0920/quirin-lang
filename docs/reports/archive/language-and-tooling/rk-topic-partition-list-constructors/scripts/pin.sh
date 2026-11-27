#!/bin/sh
# pin.sh BINARY LOGDIR CRATE FUNCTION... : pin at both audited revisions, then check the crate at both
# (round 4's scripts/pin.sh with this worktree's paths; the 66910c10 checkout
# is a shared clone of the local Fluent Bit repository, private/rk/fb-corpus).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a3bcf1839aa2e2567
BIN=$1; LOGS=$2; CRATE=$3; shift 3
FUNCS=""
for f in "$@"; do FUNCS="$FUNCS --function $f"; done
mkdir -p "$LOGS"
cd "$W" || exit 2
OLD=$W/private/rk/fb-corpus
NEW=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6/fluent-bit-clang-source
"$BIN" pin --source-root "$OLD" --crate "$CRATE" $FUNCS > "$LOGS/pin-66910c10.log" 2>&1; echo "pin 66910c10 $?"
"$BIN" pin --source-root "$NEW" --crate "$CRATE" $FUNCS > "$LOGS/pin-046accf6.log" 2>&1; echo "pin 046accf6 $?"
"$BIN" pin --source-root "$OLD" --crate "$CRATE" --check > "$LOGS/check-66910c10.log" 2>&1; echo "check 66910c10 $?"
"$BIN" pin --source-root "$NEW" --crate "$CRATE" --check > "$LOGS/check-046accf6.log" 2>&1; echo "check 046accf6 $?"
"$BIN" pin --verify-contracts > "$LOGS/verify.log" 2>&1; echo "verify $?"
