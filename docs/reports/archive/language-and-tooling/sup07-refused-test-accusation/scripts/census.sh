#!/usr/bin/env bash
# census.sh NAME -- SUP07-REFUSED-TEST-ACCUSATION's leak-witness census: build
# quirin-audit with the off-by-default `indirect-census` feature into its own
# target directory (target-census/), then run the corpus arbiter's pinned
# command, the Fluent Bit system route and the tail-pause slice with
# QCORE_LEAK_WITNESS set, each into a fresh directory under private/sup07/.
# The census binary's diagnostics must equal the plain binary's (the census
# reads only); compare the resource-diagnostics.json digests. Heavy steps run
# under the shared heavy lock; every exit goes to a file; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/sup07"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
CARGO_TARGET_DIR="$repo/target-census" flock "$lock" cargo build --release -p qcore-audit \
  --bin quirin-audit --features qcore-c-gateway/indirect-census > "$out/$name-build.log" 2>&1
echo $? > "$out/$name-build.exit"
binary="$repo/target-census/release/quirin-audit"
sha256sum "$binary" > "$out/$name.binary.sha256"
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
mkdir -p "$out/$name" || exit 2
QCORE_LEAK_WITNESS="$out/$name/leak-witness.jsonl" QCORE_CLANG=/usr/bin/clang \
  QCORE_CHUNKIO_PACKAGE="$inputs/provider-packages-wave20/qcore.chunkio" \
  flock "$lock" "$binary" pinned --jobs 4 \
  "$inputs/c-gateway-wave-pin/fluent-bit-clang-source" \
  "$inputs/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
  "$out/$name/monkey-source.json" "$out/$name/resource-diagnostics.json" \
  > "$out/$name/audit.log" 2>&1
echo $? > "$out/$name.exit"
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
mkdir -p "$out/$name-system" || exit 2
QCORE_LEAK_WITNESS="$out/$name-system/leak-witness.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
  flock "$lock" "$binary" current \
  --checkout "$prepared/fluent-bit-clang-source" \
  --profile-directory "$repo/contract/profiles/fluent-bit" \
  --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
  --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
  --jobs 4 --report "$out/$name-system/resource-diagnostics.json" \
  > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
echo $? > "$out/$name-system.exit"
QCORE_LEAK_WITNESS="$out/$name-slice-leak-witness.jsonl" \
  flock "$lock" bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "sup07-$name" \
  "$binary" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 \
  > "$out/$name-slice.log" 2>&1
echo $? > "$out/$name-slice.exit"
