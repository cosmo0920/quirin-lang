#!/usr/bin/env bash
# gate-whole.sh [NAME] -- LEAN-4-35-TOOLCHAIN: the full standing gate
# (gate.sh NAME, default g1) with one exclusive flock on the shared heavy lock
# held for the whole gate. Run as
#   flock <heavy.lock> bash gate-whole.sh [NAME]
set -u
name="${1:-g1}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/lean-4-35-toolchain" || exit 2
lockfile="$repo/private/lean-4-35-toolchain/gate-$name-lock.txt"
echo "lock held from $(date -u +%FT%T.%NZ)" > "$lockfile"
bash "$here/gate.sh" "$name"
rc=$?
echo "gate.sh exit $rc, lock released at $(date -u +%FT%T.%NZ)" >> "$lockfile"
exit "$rc"
