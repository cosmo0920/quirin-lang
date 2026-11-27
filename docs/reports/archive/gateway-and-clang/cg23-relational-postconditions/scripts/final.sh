#!/bin/sh
# final.sh NAME -- CG23-RELATIONAL-POSTCONDITIONS: the final heavy steps in
# one hold of the shared flock: the corpus AFTER on the current base
# (corpus.sh NAME), the standing gate (gate.sh private/cg23rp/gate-NAME),
# then `cargo clean --profile dev`. Exits are captured by the two scripts.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
"$here/corpus.sh" "$1"
"$here/gate.sh" "private/cg23rp/gate-$1"
cd "$repo" && cargo clean --profile dev > "$repo/private/cg23rp/clean-$1.log" 2>&1
echo $? > "$repo/private/cg23rp/clean-$1.exit"
