#!/bin/sh
# Card LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING: re-measure
# LIBCURL-PROVIDER-PLUGIN's opt-in `current` run (its callers.sh and
# compare_callers.py, unchanged) with a base and a tip binary, on a private
# preparation of Fluent Bit 046accf6 with libcurl 8.18.0's development
# headers visible to CMake.
#
#   sh rerun.sh BASE_QUIRIN_AUDIT TIP_QUIRIN_AUDIT FLUENT_BIT_CLONE LIBCURL_ROOT OUT
#
# LIBCURL_ROOT is `dpkg-deb -x libcurl4-openssl-dev_8.18.0-1ubuntu2.8_amd64.deb`.
# The user crates' checker packages must be installed first
# (`python3 providers/package.py checkers`). Every step's exit goes to a log
# under OUT; nothing is piped away.
BASE=$1
TIP=$2
FB=$3
CURL=$4
OUT=$5
cd "$(dirname "$0")/../../../.." || exit 2
W=$(pwd)
E=$W/docs/reports/libcurl-provider-plugin/evidence
mkdir -p "$OUT" || exit 2
export SOURCE_DATE_EPOCH=1767225600
CMAKE_INCLUDE_PATH=$CURL/usr/include/x86_64-linux-gnu:$CURL/usr/include \
CMAKE_LIBRARY_PATH=$CURL/usr/lib/x86_64-linux-gnu \
  "$BASE" prepare --fluent-bit "$FB" --revision 046accf6 --output "$OUT/prep" > "$OUT/prepare.log" 2>&1
echo "prepare exit $?" >> "$OUT/prepare.log"
python3 "$E/probe_database.py" "$OUT/prep/fluent-bit-clang-production/compile_commands.json" \
  "$OUT/probe" lib/librdkafka-2.15.0/src/rdhttp.c \
  lib/librdkafka-2.15.0/src/rdkafka_sasl_oauthbearer_oidc.c > "$OUT/probe.log" 2>&1
echo "probe exit $?" >> "$OUT/probe.log"
for side in base tip; do
  if [ "$side" = base ]; then binary=$BASE; else binary=$TIP; fi
  sh "$E/callers.sh" "$binary" "$OUT/prep" "$OUT/probe/selected.json" "$OUT/callers-$side" \
    > "$OUT/callers-$side.log" 2>&1
  echo "callers exit $?" >> "$OUT/callers-$side.log"
  python3 "$E/compare_callers.py" "$OUT/callers-$side" "$OUT/callers-$side/comparison.json" \
    > "$OUT/callers-$side/comparison.log" 2>&1
  echo "compare exit $?" >> "$OUT/callers-$side.log"
done
