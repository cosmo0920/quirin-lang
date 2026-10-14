#!/usr/bin/env bash
# arbiters2.sh -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE, integrator
# follow-up: AFTER2 (the release binary at the tip carrying the three clippy
# 1.99 borrow removals, built by the host's stable 1.99.0) on the corpus,
# system and slice routes with the null property selected and no parity
# stream, compared file for file with BEFORE (the base binary's runs, from
# arbiters.sh). Every exit is in private/pfw/*.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
export NULL=1 PARITY=0
bash "$here/measure-binary.sh" after2 "$repo/private/pfw/bin/tip2/quirin-audit"
AFTER=after2 python3 "$here/identity.py" > "$repo/private/pfw/identity2.txt" 2>&1
echo $? > "$repo/private/pfw/identity2.exit"
