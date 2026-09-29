#!/usr/bin/env bash
# measure.sh PHASE [ROUTES] -- card CG10-COMMON-TYPED-PLACE's arbiter
# measurements (the common brief's commands), each into a fresh directory
# under this worktree's private/cg10/PHASE. ROUTES is any of corpus, system,
# slice (default: all three, in that order). The binary is this worktree's
# target/release/quirin-audit, copied first so a later rebuild cannot change
# a running measurement's system or slice binary (the corpus runner reads
# target/release/quirin-audit itself; do not rebuild while it runs).
#   corpus: scripts/run_pinned_audit.py (66910c10), parity stream on
#   system: quirin-audit current at 046accf6 through system-selection.json
#   slice:  w47-tail-pause-bindings/scripts/run-current.sh attempt-1 cg10-PHASE
# Every exit is written to a file; nothing is piped.
set -u
phase="$1"; shift
routes="${*:-corpus system slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg10/$phase"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
[ -x "$out/quirin-audit" ] || cp target/release/quirin-audit "$out/quirin-audit" || exit 2
sha256sum "$out/quirin-audit" target/release/quirin-audit > "$out/binary.sha256"
for route in $routes; do
  case "$route" in
    corpus)
      QCORE_TYPED_PATH_PARITY="$out/corpus/parity.jsonl" python3 scripts/run_pinned_audit.py \
        "$out/corpus" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
        --jobs 8 > "$out/corpus.log" 2>&1
      echo $? > "$out/corpus.exit" ;;
    system)
      prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
      mkdir -p "$out/system" || exit 2
      QCORE_CLANG=/usr/lib/llvm-21/bin/clang "$out/quirin-audit" current \
        --checkout "$prepared/fluent-bit-clang-source" \
        --profile-directory "$repo/contract/profiles/fluent-bit" \
        --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
        --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
        --jobs 8 --report "$out/system/resource-diagnostics.json" \
        > "$out/system/console.log" 2> "$out/system/stderr.log"
      echo $? > "$out/system.exit" ;;
    slice)
      bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cg10-$phase" \
        "$out/quirin-audit" \
        --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
        > "$out/slice.log" 2>&1
      echo $? > "$out/slice.exit" ;;
  esac
done
