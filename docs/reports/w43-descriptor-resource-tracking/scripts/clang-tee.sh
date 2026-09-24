#!/bin/bash
# Transparent clang wrapper for a private capture run only: saves every
# preprocessing output (-E line-marked to pp/, -E -P to ppP/) and changes
# nothing about what the audit receives.
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
e=0; p=0
for a in "$@"; do [ "$a" = "-E" ] && e=1; [ "$a" = "-P" ] && p=1; done
if [ $e = 1 ]; then
  if [ $p = 1 ]; then OUT=$ROOT/measure/capture/ppP; else OUT=$ROOT/measure/capture/pp; fi
  mkdir -p "$OUT"
  id=$(date +%s%N)-$$
  printf '%s\n' "$@" > "$OUT/$id.args"
  /usr/bin/clang "$@" > "$OUT/$id.i"
  rc=$?
  cat "$OUT/$id.i"
  exit $rc
fi
exec /usr/bin/clang "$@"
