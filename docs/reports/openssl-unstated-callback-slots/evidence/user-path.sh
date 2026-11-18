#!/bin/sh
# Card OPENSSL-UNSTATED-CALLBACK-SLOTS: the user-crate path with the real
# binaries over the committed user load path (no file of the tree written):
# quirin-plugin check; quirin-audit plugin views (no write), pin
# --verify-contracts, and the contracts-only route on this card's two-unit
# fixture. Run after `providers/package.py checkers` installed the checker.
#
#   sh user-path.sh QUIRIN_PLUGIN QUIRIN_AUDIT OUT
set -eu
QP=$1
QA=$2
OUT=$3
cd "$(dirname "$0")/../../../.."
P=providers
FIXTURE=docs/reports/openssl-unstated-callback-slots/evidence/fixture
mkdir -p "$OUT"
run() {
  name=$1
  shift
  echo "\$ $*" > "$OUT/$name.log"
  if "$@" >> "$OUT/$name.log" 2>&1; then code=0; else code=$?; fi
  echo "exit $code" >> "$OUT/$name.log"
  tail -1 "$OUT/$name.log"
}
run 1-check "$QP" check $P/quirin-provider-openssl
run 2-views "$QA" plugin views --load-path $P/user-crates-load-path.json
run 3-verify-contracts "$QA" pin --verify-contracts --load-path $P/user-crates-load-path.json
run 4-contracts "$QA" contracts --load-path $P/user-crates-load-path.json \
  --provider quirin-provider-libc --provider quirin-provider-openssl \
  --source-root $FIXTURE --report "$OUT/r.json"
