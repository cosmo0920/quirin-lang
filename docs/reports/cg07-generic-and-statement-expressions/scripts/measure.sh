#!/usr/bin/env bash
# measure.sh NAME [corpus|system|slice ...] -- CG07-GENERIC-AND-STATEMENT-EXPRESSIONS's
# corpus arbiter, system route and tail-pause slice with this worktree's release
# quirin-audit (already built), each into a fresh directory: corpus and system
# under private/cg07/NAME*, the slice under the shared prepared attempt as
# cg07-NAME. The commands are those of
# docs/reports/cg10-bound-untracked-roots/scripts/measure.sh, with --jobs 4
# (round 7's shared-host limit). Every exit is written to a file; nothing is piped.
set -u
name="$1"; shift
routes="${*:-corpus system slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg07"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
binary="$repo/target/release/quirin-audit"
sha256sum "$binary" > "$out/$name.binary.sha256"
for route in $routes; do
  case "$route" in
    corpus)
      QCORE_TYPED_PATH_PARITY="$out/$name.parity.jsonl" python3 scripts/run_pinned_audit.py \
        "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
        --jobs 4 > "$out/$name.runner.log" 2>&1
      echo $? > "$out/$name.exit"
      ;;
    system)
      prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
      mkdir -p "$out/$name-system" || exit 2
      QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
        "$binary" current \
        --checkout "$prepared/fluent-bit-clang-source" \
        --profile-directory "$repo/contract/profiles/fluent-bit" \
        --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
        --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
        --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
        > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
      echo $? > "$out/$name-system.exit"
      ;;
    slice)
      QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl" \
        bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "cg07-$name" \
        "$binary" \
        --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
        > "$out/$name-slice.log" 2>&1
      echo $? > "$out/$name-slice.exit"
      ;;
  esac
done
