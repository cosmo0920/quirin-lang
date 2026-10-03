#!/usr/bin/env bash
# clang-census.sh -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION, measurement only: a
# QCORE_CLANG wrapper that runs the real Clang (QCORE_CLANG_REAL, default
# /usr/lib/llvm-21/bin/clang) with the same arguments, standard input and exit
# status, and appends one JSON line per launch to QCORE_CLANG_CENSUS_LOG:
# start time, wall seconds, mode, arguments (frontend flags elided to a digest),
# the bytes and sha256 of the standard input (read only when an argument is
# `-`) and of the standard output, the exit status and the parent command.
# Its own process launches add time, so census wall times are not the
# measured wall times; the counts and the duplicate-input digests are the point.
# Never used for an identity run: the wrapper path is recorded as the Clang
# program by provenance readers.
real="${QCORE_CLANG_REAL:-/usr/lib/llvm-21/bin/clang}"
log="${QCORE_CLANG_CENSUS_LOG:?QCORE_CLANG_CENSUS_LOG unset}"
tmp="$(mktemp -d "${QCORE_CLANG_CENSUS_TMP:-${TMPDIR:-/tmp}}/clang-census.XXXXXX")" || exit 125
stdin_used=0
for argument in "$@"; do [ "$argument" = - ] && stdin_used=1; done
start=$EPOCHREALTIME
if [ "$stdin_used" = 1 ]; then
  cat > "$tmp/in"
  "$real" "$@" < "$tmp/in" > "$tmp/out"
else
  "$real" "$@" > "$tmp/out"
fi
rc=$?
end=$EPOCHREALTIME
cat "$tmp/out"
mode=other
case " $* " in
  *" --version "*) mode=version;;
  *" -dumpmachine "*) mode=dumpmachine;;
  *" -dM "*) mode=macros;;
  *"-fdump-record-layouts"*) mode=record-layouts;;
  *"-ast-dump-filter"*) mode=ast-filtered;;
  *"-ast-dump=json"*) mode=ast-full;;
  *" -E "*) mode=preprocess;;
  *" -fsyntax-only "*) mode=syntax;;
esac
filter=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "-ast-dump-filter" ]; then filter="$argument"; fi
  [ "$argument" = -Xclang ] || previous="$argument"
done
in_bytes=0; in_sha=""
if [ "$stdin_used" = 1 ]; then
  in_bytes=$(stat -c %s "$tmp/in"); in_sha=$(sha256sum "$tmp/in" | cut -c1-16)
fi
out_bytes=$(stat -c %s "$tmp/out"); out_sha=$(sha256sum "$tmp/out" | cut -c1-16)
args_sha=$(printf '%s\0' "$@" | sha256sum | cut -c1-16)
parent=$(tr '\0' ' ' < /proc/$PPID/cmdline 2>/dev/null | cut -c1-160 | sed 's/\\/\\\\/g; s/"/\\"/g')
printf '{"start":%s,"seconds":%s,"mode":"%s","filter":"%s","args":"%s","in_bytes":%s,"in":"%s","out_bytes":%s,"out":"%s","exit":%s,"parent":"%s"}\n' \
  "$start" "$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.4f", e - s }')" "$mode" \
  "$(printf '%s' "$filter" | sed 's/\\/\\\\/g; s/"/\\"/g')" "$args_sha" "$in_bytes" "$in_sha" \
  "$out_bytes" "$out_sha" "$rc" "$parent" >> "$log"
rm -rf "$tmp"
exit "$rc"
