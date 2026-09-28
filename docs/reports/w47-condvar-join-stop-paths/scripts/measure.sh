#!/usr/bin/env bash
# measure.sh PHASE TREE -- the three arbiter measurements of card
# W47-CONDVAR-JOIN-STOP-PATHS (the commands of the common brief), each into a
# fresh directory under this worktree's private/sp/PHASE. TREE is the
# qcore-c tree whose target/release/quirin-audit, scripts and contract
# selection are measured: the base export (fefb9199, `git archive` into the
# session scratchpad, not under qcore-c/) for BEFORE, this worktree for AFTER.
#   corpus: TREE/scripts/run_pinned_audit.py (66910c10, 185 units)
#   system: quirin-audit current at 046accf6 through system-selection.json,
#           the command of docs/reports/w46-contract-gap-expansion/scripts/run_system.py
#           (run directly: the base export is not a git checkout)
#   slice:  TREE/docs/reports/w47-tail-pause-bindings/scripts/run-current.sh
#           attempt-1 sp-PHASE (the 90978ac6 export, tail-pause-selection.json)
# Every exit is written to a file; nothing is piped.
set -u
phase="$1"
tree="$(cd "$2" && pwd -P)"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sp/$phase"
mkdir -p "$out" || exit 2
binary="$tree/target/release/quirin-audit"
sha256sum "$binary" > "$out/binary.sha256"
cd "$tree" || exit 2
QCORE_TYPED_PATH_PARITY="$out/corpus/parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/corpus" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 8 > "$out/corpus.log" 2>&1
echo $? > "$out/corpus.exit"
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir -p "$out/system" || exit 2
QCORE_CLANG=/usr/lib/llvm-21/bin/clang "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$tree/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$tree/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 8 --report "$out/system/resource-diagnostics.json" \
  > "$out/system/console.log" 2> "$out/system/stderr.log"
echo $? > "$out/system.exit"
bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "sp-$phase" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
  > "$out/slice.log" 2>&1
echo $? > "$out/slice.exit"
