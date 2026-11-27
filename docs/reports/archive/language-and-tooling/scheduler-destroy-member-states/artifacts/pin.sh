#!/bin/sh
# pin.sh OUT [--check]: pin (after review) mk_http_thread_destroy and
# mk_thread_destroy at the two revisions mk_http_thread_destroy was pinned at
# (046accf6: the LERI card's system preparation; 66910c10: its corpus input
# tree), or with --check only check the crate at both trees and verify.
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ad32a2c41eddd6a5e
P=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ad857989f04ad049c/private
B=$R/target/release/quirin-audit
OUT=$1
cd "$R" || exit 2
mkdir -p "$OUT" || exit 2
OLDER=$P/prep/system/fluent-bit-clang-source
CORPUS=$P/corpus-inputs/c-gateway-wave-pin/fluent-bit-clang-source
if [ "$2" = "--check" ]; then
  "$B" pin --source-root "$OLDER" --check --crate qcore-provider-libevent > "$OUT/check-046accf6.log" 2>&1
  echo $? > "$OUT/check-046accf6.exit"
  "$B" pin --source-root "$CORPUS" --check --crate qcore-provider-libevent > "$OUT/check-66910c10.log" 2>&1
  echo $? > "$OUT/check-66910c10.exit"
  "$B" pin --verify-contracts > "$OUT/verify.log" 2>&1
  echo $? > "$OUT/verify.exit"
else
  "$B" pin --source-root "$OLDER" --crate qcore-provider-libevent --function mk_http_thread_destroy --function mk_thread_destroy > "$OUT/pin-046accf6.log" 2>&1
  echo $? > "$OUT/pin-046accf6.exit"
  "$B" pin --source-root "$CORPUS" --crate qcore-provider-libevent --function mk_http_thread_destroy --function mk_thread_destroy > "$OUT/pin-66910c10.log" 2>&1
  echo $? > "$OUT/pin-66910c10.exit"
fi
cat "$OUT"/*.exit
