#!/usr/bin/env bash
# route-census-run.sh NAME BINARY -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION,
# measurement only: the three routes with a `route-census` build (the
# fallback tables' consultation census), one census stream per route
# (private/gapo/route-census-ROUTE.jsonl), through measure-binary.sh (so the
# outputs are also compared with BEFORE by identity.py). Exits to files.
set -u
name="$1"; binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
for route in corpus system slice; do
  rm -f "$repo/private/gapo/route-census-$route.jsonl"
  QCORE_ROUTE_CENSUS="$repo/private/gapo/route-census-$route.jsonl" \
    "$here/measure-binary.sh" "$name" "$binary" "$route"
done
