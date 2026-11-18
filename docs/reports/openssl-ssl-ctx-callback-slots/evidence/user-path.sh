#!/bin/sh
# Card OPENSSL-SSL-CTX-CALLBACK-SLOTS (after OPENSSL-PROVIDER-REMAINING-CALLEES): the user-crate path with the real
# binaries, as OPENSSL-PROVIDER-PLUGIN's evidence/user-path.sh takes it
# (quirin-plugin check, package, verify, load-path; quirin-audit plugin
# views --write, pin, contracts, pin --verify-contracts), on this card's
# fixture. Writes the user load path and the crate's lock (committed after
# review) and the run's report under OUT (private).
#
#   sh user-path.sh QUIRIN_PLUGIN QUIRIN_AUDIT CHECKER OUT
set -eu
QP=$1
QA=$2
CHECKER=$3
OUT=$4
cd "$(dirname "$0")/../../../.."
P=providers
FIXTURE=docs/reports/openssl-ssl-ctx-callback-slots/evidence/fixture
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
rm -rf $P/quirin-provider-openssl/checkers/openssl-check
mkdir -p $P/quirin-provider-openssl/checkers
run 2-package "$QP" package --executable "$CHECKER" --output $P/quirin-provider-openssl/checkers/openssl-check
run 3-verify "$QP" verify $P/quirin-provider-openssl/checkers/openssl-check
run 4-load-path "$QP" load-path --extends contract-load-path.json \
  --output $P/user-crates-load-path.json $P/quirin-provider-openssl
run 5-views "$QA" plugin views --load-path $P/user-crates-load-path.json --write
run 6-pin "$QA" pin --load-path $P/user-crates-load-path.json --source-root $FIXTURE
run 7-contracts "$QA" contracts --load-path $P/user-crates-load-path.json \
  --provider quirin-provider-libc --provider quirin-provider-openssl \
  --source-root $FIXTURE --report "$OUT/r.json"
run 8-verify-contracts "$QA" pin --verify-contracts --load-path $P/user-crates-load-path.json
