#!/bin/bash
# Transparent clang wrapper for a private capture run only (card
# W43-INDEXED-RESOURCE-COLLECTIONS, after W43-LIBC-RESULT-ALIAS's wrapper):
# saves every `-E -P` preprocessing output to $QCORE_CAPTURE_DIR/ppP and
# changes nothing about what the audit receives.
e=0; p=0
for a in "$@"; do [ "$a" = "-E" ] && e=1; [ "$a" = "-P" ] && p=1; done
if [ $e = 1 ] && [ $p = 1 ] && [ -n "$QCORE_CAPTURE_DIR" ]; then
  OUT=$QCORE_CAPTURE_DIR/ppP
  mkdir -p "$OUT"
  id=$(date +%s%N)-$$
  printf '%s\n' "$@" > "$OUT/$id.args"
  /usr/bin/clang "$@" > "$OUT/$id.i"
  rc=$?
  cat "$OUT/$id.i"
  exit $rc
fi
exec /usr/bin/clang "$@"
