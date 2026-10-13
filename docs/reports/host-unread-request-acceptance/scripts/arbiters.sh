#!/usr/bin/env bash
# arbiters.sh -- HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: BEFORE (the base
# release binary, built at e0b92ece6's verifier host) then AFTER (this tip's),
# each on the corpus, system and slice routes with the null property selected
# and no parity stream, then the identity comparison. Saved binaries under
# private/hur/bin/; every exit is in private/hur/*.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
export NULL=1 PARITY=0
bash "$here/measure-binary.sh" before "$repo/private/hur/bin/quirin-audit-base"
bash "$here/measure-binary.sh" after "$repo/private/hur/bin/quirin-audit-after"
python3 "$here/identity.py" > "$repo/private/hur/identity.txt" 2>&1
echo $? > "$repo/private/hur/identity.exit"
