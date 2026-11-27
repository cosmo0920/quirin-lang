#!/usr/bin/env bash
# measure.sh NAME BINARY [ROUTES] -- GAPO-PARITY-DECISIONS-NONDETERMINISTIC
# (from GATEWAY-ANALYSIS-PATH-OPTIMIZATION's measure-atomic.sh): the pinned
# corpus arbiter, the system route and the tail-pause slice with a private
# copy of quirin-audit (private/gapo-nd/bin/...), each into a fresh
# directory (corpus/system under private/gapo-nd/NAME*, the slice under the
# shared prepared attempt as gapo-nd-NAME), each under the shared heavy lock,
# with the parity stream on. run_pinned_audit.py always runs
# target/release/quirin-audit: BINARY's bytes are written there inside the
# same lock acquisition as the run, so no other build can replace them in
# between; run.json's binary_sha256 is checked against BINARY afterwards
# (NAME.binary-check). ROUTES is a comma list of corpus,system,slice (default
# all). PARITY=0 runs without the parity stream. Every exit is written to a
# file; nothing is piped.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; routes="${3:-corpus,system,slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/gapo-nd"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
parity() {
  # parity FILE: export the parity stream's path unless PARITY=0
  if [ "${PARITY:-1}" != 0 ]; then export QCORE_TYPED_PATH_PARITY="$1"; else unset QCORE_TYPED_PATH_PARITY; fi
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
  ( parity "$out/$name-parity.jsonl"; timed "$out/$name.seconds" "$out/$name.exit" "$out/$name.log" "$out/$name.stderr" \
    /usr/bin/env bash -c 'cat "$1" > "$2" && sha256sum "$2" > "$3" && shift 3 && exec "$@"' _ \
    "$binary" "$repo/target/release/quirin-audit" "$out/$name.corpus-binary.sha256" \
    python3 scripts/run_pinned_audit.py "$out/$name" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 )
  python3 -c '
import json, sys
want = open(sys.argv[1]).read().split()[0]
got = json.load(open(sys.argv[2])).get("binary_sha256")
print("OK" if got == want else f"MISMATCH run.json {got} binary {want}")
' "$out/$name.binary.sha256" "$out/$name/run.json" > "$out/$name.binary-check" 2>&1;;
esac
case ",$routes," in *,system,*)
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/$name-system" || exit 2
  ( parity "$out/$name-system-parity.jsonl"; export QCORE_CLANG="${SYSTEM_CLANG:-/usr/lib/llvm-21/bin/clang}"
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
  ( parity "$out/$name-slice-parity.jsonl"
    timed "$out/$name-slice.seconds" "$out/$name-slice.exit" \
    "$out/$name-slice.log" "$out/$name-slice.stderr" \
    bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "gapo-nd-$name" \
    "$binary" \
    --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 4 );;
esac
