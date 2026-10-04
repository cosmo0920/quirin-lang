#!/usr/bin/env bash
# before-unit.sh OUT-DIR -- GAPO-PARITY-DECISIONS-NONDETERMINISTIC: run the
# minimised BEFORE test (before_unit_test.rs) three times with the parity
# stream on. Before it, from a clean tree at the card's tip:
#   checkout 0ebe77f86 -- crates/qcore-c-gateway/src crates/qcore-c-gateway/tests/unit/typed_place.rs
# and after it:
#   checkout HEAD -- crates/qcore-c-gateway
# (the base's gateway sources over the working tree, then the card's back).
# Exits to files; nothing piped.
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
mkdir -p "$out" || exit 2
cat "$here/before_unit_test.rs" >> crates/qcore-c-gateway/tests/unit/typed_place.rs
for run in 1 2 3; do
  QCORE_TYPED_PATH_PARITY="$out/parity-$run.jsonl" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 \
    cargo test -p qcore-c-gateway --lib -- --exact \
    typed_place::tests::gapo_nd_before_a_declined_consultation_holds_no_leftover \
    > "$out/run-$run.log" 2>&1
  echo $? > "$out/run-$run.exit"
done
