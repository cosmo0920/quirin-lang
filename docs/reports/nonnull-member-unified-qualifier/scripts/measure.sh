#!/usr/bin/env bash
# measure.sh NAME BINARY [ROUTES] -- NONNULL-MEMBER-UNIFIED-QUALIFIER (from FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE) (from FB-QUANT-CONTRACT-CENSUS, adapted from
# NULL-TRIAGE-UNRESOLVED-PATHS's scripts/measure.sh and measure-binary.sh):
# one run per route with BINARY and the null property
# (QCORE_NULL_PROPERTY), no parity stream. ROUTES is a comma list of
# corpus,system,slice (default all). Each route runs under the shared heavy
# lock with its wall time measured inside the lock into NAME[-route].seconds.
# The corpus arbiter runs target/release/quirin-audit, so BINARY's bytes are
# written there first (under the lock) and run.json's binary_sha256 checks it.
# Outputs: private/nmuq/NAME* (corpus, system) and the shared prepared slice
# attempt private/w47-tail/attempt-1/nmuq-NAME. Every exit to a file; nothing
# is piped.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; routes="${3:-corpus,system,slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/nmuq"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
timed() {
  # timed SECONDS-FILE EXIT-FILE STDOUT STDERR COMMAND... : run under the lock, time inside it
  local seconds="$1" exitfile="$2" stdout="$3" stderr="$4"; shift 4
  flock "$lock" /usr/bin/env bash -c '
    seconds="$1" exitfile="$2" stdout="$3" stderr="$4"; shift 4
    start=$EPOCHREALTIME
    "$@" > "$stdout" 2> "$stderr"
    rc=$?
    end=$EPOCHREALTIME
    echo "$rc" > "$exitfile"
    awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$seconds"
  ' _ "$seconds" "$exitfile" "$stdout" "$stderr" "$@"
}
case ",$routes," in *,corpus,*)
  mkdir -p "$repo/target/release" || exit 2
  flock "$lock" /usr/bin/env bash -c 'cat "$1" > "$2" && chmod +x "$2" && sha256sum "$2" > "$3"' _ \
    "$binary" "$repo/target/release/quirin-audit" "$out/$name.corpus-binary.sha256" || exit 3
  ( unset QCORE_TYPED_PATH_PARITY
    export QCORE_NULL_PROPERTY="$out/$name-null.jsonl"
    timed "$out/$name.seconds" "$out/$name.exit" "$out/$name.log" "$out/$name.stderr" \
      python3 scripts/run_pinned_audit.py "$out/$name" 66910c10 \
      --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 );;
esac
case ",$routes," in *,system,*)
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/$name-system" || exit 2
  ( unset QCORE_TYPED_PATH_PARITY
    export QCORE_NULL_PROPERTY="$out/$name-system-null.jsonl"
    QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
    timed "$out/$name-system.seconds" "$out/$name-system.exit" \
      "$out/$name-system/console.log" "$out/$name-system/stderr.log" \
      "$binary" current \
      --checkout "$prepared/fluent-bit-clang-source" \
      --profile-directory "$repo/contract/profiles/fluent-bit" \
      --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
      --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
      --jobs 4 --report "$out/$name-system/resource-diagnostics.json" );;
esac
case ",$routes," in *,slice,*)
  ( unset QCORE_TYPED_PATH_PARITY
    export QCORE_NULL_PROPERTY="$out/$name-slice-null.jsonl"
    timed "$out/$name-slice.seconds" "$out/$name-slice.exit" \
      "$out/$name-slice.log" "$out/$name-slice.stderr" \
      bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "nmuq-$name" \
      "$binary" \
      --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 );;
esac
