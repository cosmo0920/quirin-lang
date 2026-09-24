#!/bin/sh
# Declared-ABI census over the captured, line-marker-stripped units, with the
# base reader (a7a936d9) and this card's reader, for three contract sets.
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
cd "$ROOT" || exit 2
OLD=measure/base-target/release/examples/declared_abi_census
NEW=target/release/examples/declared_abi_census
OUT=measure/census
mkdir -p $OUT
for tool in old new; do
  if [ $tool = old ]; then BIN=$OLD; else BIN=$NEW; fi
  $BIN measure/base-src/cycle19-resource-contracts.json measure/capture/plain/*.i > $OUT/manifest-$tool.txt 2>&1
  echo "manifest-$tool $?" >> $OUT/exits.txt
  $BIN docs/reports/w43-libc-result-alias/measurement/new13.qcore -- measure/capture/plain/*.i > $OUT/new13-$tool.txt 2>&1
  echo "new13-$tool $?" >> $OUT/exits.txt
  $BIN docs/reports/w43-libc-result-alias/measurement/restrict14.qcore -- measure/capture/plain/*.i > $OUT/restrict14-$tool.txt 2>&1
  echo "restrict14-$tool $?" >> $OUT/exits.txt
done
cat $OUT/exits.txt
