#!/usr/bin/env bash
# after.sh -- CG29-EXECUTION-COVERAGE-AND-RECLAMATION on the rebased tip:
# workspace check, the card's own targets, then the AFTER release binary and
# the corpus / system / slice measurement (measure.sh). Each exit is written
# to a file under private/cg29; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg29"
cd "$repo" || exit 2
git submodule update --init --recursive > "$out/after-submodule.log" 2>&1
echo $? > "$out/after-submodule.exit"
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 cargo check --workspace --all-targets > "$out/after-check.log" 2>&1
echo $? > "$out/after-check.exit"
bash "$here/targeted.sh" t2
bash "$here/measure.sh" after
