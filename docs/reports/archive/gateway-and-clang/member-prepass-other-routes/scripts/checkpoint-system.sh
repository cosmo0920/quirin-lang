#!/usr/bin/env bash
# checkpoint-system.sh NAME BINARY -- MEMBER-PREPASS-OTHER-ROUTES: the system
# route twice with BINARY, without the null property (checkpoints refuse it):
# whole (private/mpor/NAME-sysw) and with --checkpoint-dir
# (private/mpor/NAME-sysc, its .qo files in NAME-sysc/qo). Each run under the
# shared heavy lock, timed inside it; exits and wall times to files. The two
# reports and sidecars are compared byte for byte by the report.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpor"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
cd "$repo" || exit 2
unset QCORE_NULL_PROPERTY QCORE_BOUNDS_PROPERTY
run() {
  local label="$1"; shift
  mkdir -p "$out/$name-$label" || exit 2
  QCORE_CLANG=/usr/lib/llvm-21/bin/clang flock "$lock" /usr/bin/env bash -c '
    dir="$1"; shift
    start=$EPOCHREALTIME
    "$@" > "$dir/console.log" 2> "$dir/stderr.log"
    echo $? > "$dir/exit"
    end=$EPOCHREALTIME
    awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$dir/seconds"
  ' _ "$out/$name-$label" "$binary" current \
    --checkout "$prepared/fluent-bit-clang-source" \
    --profile-directory "$repo/contract/profiles/fluent-bit" \
    --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
    --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
    --jobs 4 --report "$out/$name-$label/resource-diagnostics.json" "$@"
}
run sysw
run sysc --checkpoint-dir "$out/$name-sysc/qo"
