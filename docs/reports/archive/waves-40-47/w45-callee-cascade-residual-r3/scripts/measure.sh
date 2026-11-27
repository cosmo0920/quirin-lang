#!/usr/bin/env bash
# measure.sh NAME [ROUTES] -- W45-CALLEE-CASCADE-RESIDUAL round 3 (from NULL-TRIAGE-UNRESOLVED-PATHS and its lineage) (adapted from
# CG22-CALLBACK-REGISTRY-CONTRACT's, itself from CG10-INDIRECT-ACCESS-SOLE-
# BLOCKERS's scripts/measure.sh): the corpus arbiter, system route and
# tail-pause slice with this worktree's release quirin-audit, each into a fresh
# directory (corpus/system under private/w45r3/NAME*, the slice under the shared
# prepared attempt as w45r3-NAME), each under the shared heavy lock, with the
# route's wall time measured inside the lock (lock waiting is not counted)
# into NAME[-system|-slice].seconds. ROUTES is a comma list of
# corpus,system,slice (default all). NULL=1 also selects the null property
# (QCORE_NULL_PROPERTY), its rows into NAME[-system|-slice]-null.jsonl;
# PARITY=0 skips the parity stream. Every exit is written to a file; nothing
# is piped. The binary must already be at target/release/quirin-audit (BINARY
# overrides it for the system and slice routes; the corpus arbiter always runs
# target/release/quirin-audit).
set -u
name="$1"
routes="${2:-corpus,system,slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/w45r3"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
binary="${BINARY:-$repo/target/release/quirin-audit}"
sha256sum "$binary" > "$out/$name.binary.sha256"
selections() {
  # selections RUN: the selection variables of one route, exported.
  unset QCORE_TYPED_PATH_PARITY QCORE_NULL_PROPERTY
  if [ "${PARITY:-1}" = 1 ]; then export QCORE_TYPED_PATH_PARITY="$out/$1-parity.jsonl"; fi
  if [ "${NULL:-0}" = 1 ]; then export QCORE_NULL_PROPERTY="$out/$1-null.jsonl"; fi
}
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
  ( selections "$name"
    timed "$out/$name.seconds" "$out/$name.exit" \
      "$out/$name.log" "$out/$name.stderr" \
      python3 scripts/run_pinned_audit.py "$out/$name" 66910c10 \
      --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 );;
esac
case ",$routes," in *,system,*)
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/$name-system" || exit 2
  ( selections "$name-system"
    QCORE_CLANG="${SYSTEM_CLANG:-/usr/lib/llvm-21/bin/clang}" \
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
  ( selections "$name-slice"
    timed "$out/$name-slice.seconds" "$out/$name-slice.exit" \
      "$out/$name-slice.log" "$out/$name-slice.stderr" \
      bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "w45r3-$name" \
      "$binary" \
      --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 );;
esac
