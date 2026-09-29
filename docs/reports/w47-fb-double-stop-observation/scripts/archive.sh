#!/usr/bin/env bash
# archive.sh -- copy attempt-1's summaries, identity files, gdb logs and the
# first run of each scenario per campaign into the report's artifacts/.
# Every other run log stays private; its SHA-256 is in the summary.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
art="$here/../artifacts"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1
mkdir -p "$art/logs" "$art/identity" || exit 2
for c in runs-90978ac6-debug runs-046accf6-debug runs-90978ac6-tsan; do
  cp -- "$root/$c/summary.json" "$art/$c.summary.json" || exit 3
  for s in control v i iii; do
    cp -- "$root/$c/$s-01.log" "$art/logs/$c.$s-01.log" || exit 3
    cp -- "$root/$c/$s-01.cmd" "$art/logs/$c.$s-01.cmd" || exit 3
  done
done
for t in 90978ac6 046accf6; do
  cp -- "$root/gdb-$t/summary.json" "$art/gdb-$t.summary.json" || exit 3
  for s in control v i iii; do
    cp -- "$root/gdb-$t/gdb-$s.log" "$art/logs/gdb-$t.$s.log" || exit 3
  done
done
cp -- "$root/identity/toolchain.txt" "$root/identity/binaries.sha256" \
      "$root"/identity/manifest-*.digest "$root"/identity/manifest-*.count \
      "$root/compare/compare.txt" "$art/identity/" || exit 3
for f in "$root"/logs/*.exit; do printf '%s %s\n' "$(basename "$f")" "$(cat "$f")"; done > "$art/identity/build-exits.txt"
sed -i "s#$root/#attempt-1/#" "$art/identity/binaries.sha256"
echo archived
