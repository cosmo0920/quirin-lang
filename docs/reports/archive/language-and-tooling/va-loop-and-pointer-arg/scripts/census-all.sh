#!/usr/bin/env bash
# census-all.sh NAME -- the census and digests of one measurement NAME (corpus,
# system, tail-pause slice) into private/valoop/census-NAME-*.json.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/valoop"
cd "$repo" || exit 2
corpus="$out/$name/resource-diagnostics.json"
system="$out/$name-system/resource-diagnostics.json"
slice="/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/valoop-$name/resource-diagnostics.json"
sha256sum "$corpus" "$system" "$slice" > "$out/$name-digests.txt"
cat "$out/$name-digests.txt"
cycle=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin
python3 "$here/census.py" "$corpus" "$cycle/fluent-bit-clang-source/lib" "$cycle/monkey/monkey-source" \
  "$out/census-$name-corpus.json"
python3 "$here/census.py" "$system" \
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source \
  "$out/census-$name-system.json"
python3 "$here/census.py" "$slice" \
  /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source \
  "$out/census-$name-slice.json"
