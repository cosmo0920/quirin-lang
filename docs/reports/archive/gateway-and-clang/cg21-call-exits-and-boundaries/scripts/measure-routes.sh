#!/usr/bin/env bash
# measure-routes.sh NAME BINARY [TREE] -- CG21-CALL-EXITS-AND-BOUNDARIES: the
# system route and the tail-pause slice alone (measure.sh's two `current`
# runs), reading the contract profile from TREE (default: this worktree).
#
# Why: `quirin-audit current` reads the contract modules from the tree it is
# pointed at when it runs, so a BEFORE binary over this card's tree rejects
# `resource_no_return()`. The BEFORE runs therefore point TREE at an
# extraction of the base commit's `contract/`, `providers/` and the slice
# runner (`git archive b6a3dc42 contract providers
# docs/reports/w47-tail-pause-bindings/scripts`, under private/cg21/basetree).
# Outputs go to this worktree's private/cg21 (system) and to the shared
# prepared attempt as cg21-NAME (slice). Every exit is written to a file.
set -u
name="$1"
binary="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
tree="${3:-$repo}"
out="$repo/private/cg21"
mkdir -p "$out" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir -p "$out/$name-system" || exit 2
cd "$tree" || exit 2
QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$tree/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$tree/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
  > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
echo $? > "$out/$name-system.exit"
QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl" \
  bash "$tree/docs/reports/w47-tail-pause-bindings/scripts/run-current.sh" attempt-1 "cg21-$name" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
