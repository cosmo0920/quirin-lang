#!/usr/bin/env bash
# measure.sh PHASE TREE -- the three arbiter measurements of card
# W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL (the common brief's commands, as
# docs/reports/w47-condvar-join-stop-paths/scripts/measure.sh runs them), each
# into a fresh directory under this worktree's private/rh/PHASE. TREE is the
# qcore-c tree whose target/release/quirin-audit, scripts and contract
# selection are measured: the base export (fefb9199, an archive of the commit
# unpacked into the session scratchpad, not under qcore-c/) for BEFORE, this
# worktree for AFTER.
#   corpus: TREE/scripts/run_pinned_audit.py (66910c10, 185 units)
#   system: quirin-audit current at 046accf6 through system-selection.json
#   slice:  TREE/docs/reports/w47-tail-pause-bindings/scripts/run-current.sh
#           attempt-1 rh-PHASE (the 90978ac6 export, tail-pause-selection.json)
# ROUTES (default "corpus system slice") names the routes to run. The base
# export has an empty third_party/libevent (an archive leaves the submodule
# out) and is not a checkout: link third_party/libevent to a checkout of the
# pinned revision first, since the corpus route verifies that pin. The release
# binary is built first (exit recorded). Every exit is written to a file;
# nothing is piped.
set -u
phase="$1"
tree="$(cd "$2" && pwd -P)"
routes=" ${ROUTES:-corpus system slice} "
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rh/$phase"
mkdir "$out" || exit 2
cd "$tree" || exit 2
CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit \
  > "$out/build.log" 2>&1
echo $? > "$out/build.exit"
binary="$tree/target/release/quirin-audit"
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
    --profile-directory "$tree/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$tree/contract/profiles/fluent-bit/system-selection.json" \
    --jobs 8 --report "$out/system/resource-diagnostics.json" \
    > "$out/system/console.log" 2> "$out/system/stderr.log"
  echo $? > "$out/system.exit"
fi
if [[ "$routes" == *" slice "* ]]; then
  bash "$tree/docs/reports/w47-tail-pause-bindings/scripts/run-current.sh" attempt-1 "rh-$phase" \
    "$binary" \
    --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
    > "$out/slice.log" 2>&1
  echo $? > "$out/slice.exit"
fi
