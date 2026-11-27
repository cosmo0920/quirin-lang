#!/bin/sh
# Declared-ABI census over the captured, line-marker-stripped units, with the
# base reader (f8b41209, measure/base-bin) and this card's reader
# (target/release), for one contract set.
#
# Usage: census.sh LABEL MANIFEST.json
#        census.sh LABEL MODULE.qcore... --
# Writes measure/census/LABEL-{old,new}.txt and appends both exits to
# measure/census/exits.txt. Each exit is captured to a file, never piped.
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
cd "$ROOT" || exit 2
LABEL=$1
shift
OUT=measure/census
mkdir -p $OUT
for tool in old new; do
  if [ $tool = old ]; then BIN=measure/base-bin/declared_abi_census; else BIN=target/release/examples/declared_abi_census; fi
  if [ "${1%.json}" != "$1" ]; then
    $BIN "$1" measure/capture/plain/*.i > $OUT/$LABEL-$tool.txt 2>&1
  else
    $BIN "$@" measure/capture/plain/*.i > $OUT/$LABEL-$tool.txt 2>&1
  fi
  echo "$LABEL-$tool $?" >> $OUT/exits.txt
done
tail -2 $OUT/exits.txt
