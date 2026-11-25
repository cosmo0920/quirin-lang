#!/bin/sh
# Card ONIGMO-PROVIDER-PLUGIN: the user-crate path, run once with the real
# binaries, exactly as a user takes it: quirin-plugin check, package,
# load-path (the three user crates, named in the sorted order `contracts.py
# check` expects the checker bindings in); quirin-audit plugin views --write,
# pin (this crate only), pin --verify-contracts. Writes the user load path and
# the crate's lock (committed after review) and the run's logs under OUT
# (private).
#
#   sh user-path.sh QUIRIN_PLUGIN QUIRIN_AUDIT CHECKER SOURCE_ROOT PROBE_DATABASE OUT
#
# SOURCE_ROOT is the audited Fluent Bit checkout (046accf6), whose
# lib/onigmo holds the bodies the lock pins; PROBE_DATABASE is the compile
# database whose include configurations the version probe reads
# (QCORE_LIBRARY_VERSION_COMPILE_DATABASE): the selected units' commands,
# every one passing -I .../lib/onigmo.
set -eu
QP=$1
QA=$2
CHECKER=$3
SOURCE_ROOT=$4
PROBE_DATABASE=$5
OUT=$6
cd "$(dirname "$0")/../../../.."
P=providers
mkdir -p "$OUT"
run() {
  name=$1
  shift
  echo "\$ $*" > "$OUT/$name.log"
  if "$@" >> "$OUT/$name.log" 2>&1; then code=0; else code=$?; fi
  echo "exit $code" >> "$OUT/$name.log"
  tail -1 "$OUT/$name.log"
}
run 1-check "$QP" check $P/quirin-provider-onigmo
rm -rf $P/quirin-provider-onigmo/checkers/onigmo-check
mkdir -p $P/quirin-provider-onigmo/checkers
run 2-package "$QP" package --executable "$CHECKER" --output $P/quirin-provider-onigmo/checkers/onigmo-check
run 3-verify "$QP" verify $P/quirin-provider-onigmo/checkers/onigmo-check
run 4-load-path "$QP" load-path --extends contract-load-path.json \
  --output $P/user-crates-load-path.json $P/quirin-provider-libcurl $P/quirin-provider-onigmo \
  $P/quirin-provider-openssl
run 5-views "$QA" plugin views --load-path $P/user-crates-load-path.json --write
QCORE_LIBRARY_VERSION_COMPILE_DATABASE=$PROBE_DATABASE
export QCORE_LIBRARY_VERSION_COMPILE_DATABASE
run 6-pin "$QA" pin --load-path $P/user-crates-load-path.json --source-root "$SOURCE_ROOT" \
  --crate quirin-provider-onigmo
run 7-verify-contracts "$QA" pin --verify-contracts --load-path $P/user-crates-load-path.json
