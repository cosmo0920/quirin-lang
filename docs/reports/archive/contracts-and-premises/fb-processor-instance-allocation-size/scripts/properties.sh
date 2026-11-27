#!/usr/bin/env bash
# properties.sh OUT -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: the existing
# custody, bounds and null properties over src/flb_processor.c alone
# (scripts/processor-selection.json), with this worktree's release
# quirin-audit copied to OUT first, at the system tree (046accf6) and the
# tail-pause export (34155955), each under the shared heavy lock, each exit to
# a file. No analysis code is changed; the properties are selected by their
# environment variables (QCORE_BOUNDS_PROPERTY, QCORE_NULL_PROPERTY).
set -u
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cp "$repo/target/release/quirin-audit" "$out/quirin-audit" || exit 2
sha256sum "$out/quirin-audit" > "$out/binary.sha256"
run() {
  name="$1" prepared="$2"
  mkdir -p "$out/$name" || exit 2
  QCORE_BOUNDS_PROPERTY="$out/$name/bounds.jsonl" QCORE_NULL_PROPERTY="$out/$name/null.jsonl" \
  QCORE_CLANG=/usr/lib/llvm-21/bin/clang flock "$lock" "$out/quirin-audit" current \
    --checkout "$prepared/fluent-bit-clang-source" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$here/processor-selection.json" \
    --jobs 2 --report "$out/$name/resource-diagnostics.json" \
    > "$out/$name/console.log" 2> "$out/$name/stderr.log"
  echo $? > "$out/$name.exit"
}
run system-046accf6 /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
run slice-34155955 /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
