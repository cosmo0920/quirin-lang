#!/bin/bash
# Transparent clang wrapper for a private capture run only (card
# W44-CORPUS-CALLEE-SUMMARIES, after W43-INDEXED-RESOURCE-COLLECTIONS's
# wrapper): saves every `-E` preprocessing output (line-marked, and `-P`) to
# $QCORE_CAPTURE_DIR/pp and changes nothing about what the audit receives.
e=0; p=0
for a in "$@"; do [ "$a" = "-E" ] && e=1; [ "$a" = "-P" ] && p=1; done
if [ $e = 1 ] && [ -n "$QCORE_CAPTURE_DIR" ]; then
  OUT=$QCORE_CAPTURE_DIR/pp
  mkdir -p "$OUT"
  id=$(date +%s%N)-$$-p$p
  printf '%s\n' "$@" > "$OUT/$id.args"
  /usr/bin/clang "$@" > "$OUT/$id.i"
  rc=$?
  cat "$OUT/$id.i"
  exit $rc
fi
exec /usr/bin/clang "$@"
