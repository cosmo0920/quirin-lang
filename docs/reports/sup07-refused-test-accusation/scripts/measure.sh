#!/usr/bin/env bash
# measure.sh NAME -- SUP07-REFUSED-TEST-ACCUSATION's corpus arbiter, Fluent Bit
# system route and tail-pause slice with this worktree's release quirin-audit
# (built beforehand into target/release), each into a fresh directory
# (corpus/system under private/sup07/NAME*, the slice under the shared prepared
# attempt as sup07-NAME). Commands are those of
# docs/reports/cg05-elaborator-call-footprints/scripts/measure.sh. Each heavy
# step runs under the shared heavy lock; every exit goes to a file; nothing is
# piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/sup07"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
binary="$repo/target/release/quirin-audit"
sha256sum "$binary" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" flock "$lock" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir -p "$out/$name-system" || exit 2
QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  flock "$lock" "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
  > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
echo $? > "$out/$name-system.exit"
QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl" \
  flock "$lock" bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "sup07-$name" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
