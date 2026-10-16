#!/usr/bin/env bash
# gate-whole.sh [NAME] -- UNION-OPAQUE-RECORD-WRITES-BY-TYPE (from HANDOFF-DIRECT-CALL-READING, from HANDOFF-CANDIDATE-OWN-CALLBACKS, from OP-CALLBACK-MEMBER-PATH-RELEASE, from SUPPLIED-OWNED-RETURN-CODE-HANDOFF, from OP-APP-RK-CALLBACKS-VIA-POLL-CB) (from OP-CALLBACK-CANDIDATE-CONTRACTS, from OP-CONSTRUCTOR-DECLARATIONS, from CG22-OP-CALLBACKS-AND-SERVED-QUEUES's, from PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS's, from
# PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE's gate-g2.sh): the full standing
# gate and the provider gate (gate.sh NAME all, NAME default g1) with
# one exclusive flock on the shared heavy lock held for the whole gate, so no
# other card's heavy step runs inside it. Run as
#   flock <heavy.lock> bash gate-whole.sh [NAME]
# The lock's hold times go to private/uorw/gate-NAME-lock.txt.
set -u
name="${1:-g1}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/uorw" || exit 2
echo "lock held from $(date -u +%FT%T.%NZ)" > "$repo/private/uorw/gate-$name-lock.txt"
HELD=1 bash "$here/gate.sh" "$name" all
rc=$?
echo "gate.sh exit $rc, lock released at $(date -u +%FT%T.%NZ)" >> "$repo/private/uorw/gate-$name-lock.txt"
exit "$rc"
