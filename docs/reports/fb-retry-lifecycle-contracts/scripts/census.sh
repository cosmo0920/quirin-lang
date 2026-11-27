#!/bin/sh
# census.sh LABEL REPORT [BASE_REPORT] -- regenerate CENSUS.md and census.json
# beside this directory from one fluent-bit-system report (and, for the tip,
# the base report it is compared with), over this host's 046accf6 preparation.
D=$(cd "$(dirname "$0")/.." && pwd)
SRC=${FB_SOURCE:-/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6/fluent-bit-clang-source}
set -- --label "$1" --report "$2" ${3:+--base-report "$3"}
exec python3 "$D/scripts/census.py" "$@" --source "$SRC" \
  --out-md "$D/CENSUS.md" --out-json "$D/census.json"
