#!/bin/sh
# pin.sh OUT [--check]: pin (after review) mk_http_thread_destroy at the two
# revisions it is pinned at (046accf6: private/prep-older; 66910c10: the
# corpus input tree), or with --check check the crate at both and verify.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a4697725f5e3891df
B=$W/target/release/quirin-audit
OUT=$1
cd "$W" || exit 2
mkdir -p "$OUT" || exit 2
OLDER=$W/private/prep-older/fluent-bit-clang-source
CORPUS=$W/private/corpus-inputs/c-gateway-wave-pin/fluent-bit-clang-source
if [ "$2" = "--check" ]; then
  "$B" pin --source-root "$OLDER" --check --crate qcore-provider-libevent > "$OUT/check-046accf6.log" 2>&1
  echo $? > "$OUT/check-046accf6.exit"
  "$B" pin --source-root "$CORPUS" --check --crate qcore-provider-libevent > "$OUT/check-66910c10.log" 2>&1
  echo $? > "$OUT/check-66910c10.exit"
  "$B" pin --verify-contracts > "$OUT/verify.log" 2>&1
  echo $? > "$OUT/verify.exit"
else
  "$B" pin --source-root "$OLDER" --crate qcore-provider-libevent --function mk_http_thread_destroy > "$OUT/pin-046accf6.log" 2>&1
  echo $? > "$OUT/pin-046accf6.exit"
  "$B" pin --source-root "$CORPUS" --crate qcore-provider-libevent --function mk_http_thread_destroy > "$OUT/pin-66910c10.log" 2>&1
  echo $? > "$OUT/pin-66910c10.exit"
fi
cat "$OUT"/*.exit
