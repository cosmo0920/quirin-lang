#!/usr/bin/env bash
# arbiters.sh -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE (from HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING): BEFORE (the base
# release binary, built at b565f79ab plus the frozen prediction) then AFTER
# (this tip's, with its own HEAD commit embedded),
# each on the corpus, system and slice routes with the null property selected
# and no parity stream, then the identity comparison. Saved binaries under
# private/pfw/bin/; every exit is in private/pfw/*.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
export NULL=1 PARITY=0
bash "$here/measure-binary.sh" before "$repo/private/pfw/bin/base/quirin-audit"
bash "$here/measure-binary.sh" after "$repo/private/pfw/bin/tip/quirin-audit"
python3 "$here/identity.py" > "$repo/private/pfw/identity.txt" 2>&1
echo $? > "$repo/private/pfw/identity.exit"
