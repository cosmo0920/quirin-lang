#!/usr/bin/env bash
# measure.sh PHASE -- the three arbiter measurements of card
# W47-REVIEW-HOLD-CONCRETE-REASON (the common brief's commands, as
# docs/reports/w45-review-hold-preempts-statement-refusal/scripts/measure.sh
# runs them), from this worktree, each into a fresh directory under
# private/cr/PHASE (the slice under the prepared attempt's cr-PHASE).
#   corpus: scripts/run_pinned_audit.py (66910c10, 185 units)
#   system: quirin-audit current at 046accf6 through system-selection.json
#   slice:  docs/reports/w47-tail-pause-bindings/scripts/run-current.sh
#           attempt-1 cr-PHASE (the 90978ac6 export, tail-pause-selection.json)
# BEFORE is this worktree at its base commit, AFTER at its tip. The release
# binary is built first (exit recorded). Every exit is written to a file;
# nothing is piped. ROUTES (default "corpus system slice") selects routes.
set -u
phase="$1"
routes=" ${ROUTES:-corpus system slice} "
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cr/$phase"
mkdir -p "$repo/private/cr"
mkdir "$out" || exit 2
cd "$repo" || exit 2
CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit \
  > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
binary="$repo/target/release/quirin-audit"
sha256sum "$binary" > "$out/binary.sha256"
if [[ "$routes" == *" corpus "* ]]; then
  QCORE_TYPED_PATH_PARITY="$out/corpus/parity.jsonl" python3 scripts/run_pinned_audit.py \
    "$out/corpus" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
    --jobs 8 > "$out/corpus.log" 2>&1
  echo $? > "$out/corpus.exit"
fi
if [[ "$routes" == *" system "* ]]; then
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/system" || exit 2
  QCORE_CLANG=/usr/lib/llvm-21/bin/clang "$binary" current \
    --checkout "$prepared/fluent-bit-clang-source" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
    --jobs 8 --report "$out/system/resource-diagnostics.json" \
    > "$out/system/console.log" 2> "$out/system/stderr.log"
  echo $? > "$out/system.exit"
fi
if [[ "$routes" == *" slice "* ]]; then
  bash "$repo/docs/reports/w47-tail-pause-bindings/scripts/run-current.sh" attempt-1 "cr-$phase" \
    "$binary" \
    --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
    > "$out/slice.log" 2>&1
  echo $? > "$out/slice.exit"
fi
