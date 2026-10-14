#!/usr/bin/env bash
# gate-g2.sh -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE, integrator
# follow-up: the full standing gate and the provider gate (gate.sh g2 all) with
# one exclusive flock on the shared heavy lock held for the whole gate, so no
# other card's heavy step runs inside it. Run as
#   flock <heavy.lock> bash gate-g2.sh
# The lock's own wait and hold times go to private/pfw/gate-g2-lock.txt.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/pfw" || exit 2
echo "lock held from $(date -u +%FT%T.%NZ)" > "$repo/private/pfw/gate-g2-lock.txt"
HELD=1 bash "$here/gate.sh" g2 all
rc=$?
echo "gate.sh exit $rc, lock released at $(date -u +%FT%T.%NZ)" >> "$repo/private/pfw/gate-g2-lock.txt"
exit "$rc"
