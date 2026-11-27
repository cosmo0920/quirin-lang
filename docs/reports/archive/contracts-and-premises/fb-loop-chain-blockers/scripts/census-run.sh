#!/usr/bin/env bash
# census-run.sh NAME BINARY [ROUTES] -- FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS:
# one census run per route (system, slice) with an `indirect-census` build
# (scripts/build.sh NAME qcore-c-gateway/indirect-census): the census stream
# QCORE_INDIRECT_CENSUS (the gate's own token view of every refused
# statement) and the typed-path parity stream QCORE_TYPED_PATH_PARITY (the
# Clang reader's answer beside every refused position) and the record-body
# stream QCORE_INDIRECT_CENSUS_RECORDS (which record bodies the lexical reader
# refused, and on which token). Measurement only:
# no stream is read by any decision. Each route runs under the shared
# heavy lock. Outputs: private/fblb/NAME-system/ and the shared prepared slice
# attempt private/w47-tail/attempt-1/fblb-NAME, with the streams in
# private/fblb/NAME-{system,slice}-{census,parity}.jsonl. Every exit to a
# file; nothing is piped.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; routes="${3:-system,slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fblb"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
sha256sum "$binary" > "$out/$name.binary.sha256"
case ",$routes," in *,system,*)
  prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
  mkdir -p "$out/$name-system" || exit 2
  ( unset QCORE_NULL_PROPERTY
    export QCORE_INDIRECT_CENSUS="$out/$name-system-census.jsonl"
    export QCORE_TYPED_PATH_PARITY="$out/$name-system-parity.jsonl"
    export QCORE_INDIRECT_CENSUS_RECORDS="$out/$name-system-records.jsonl"
    QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
    flock "$lock" "$binary" current \
      --checkout "$prepared/fluent-bit-clang-source" \
      --profile-directory "$repo/contract/profiles/fluent-bit" \
      --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
      --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
      --jobs 1 --report "$out/$name-system/resource-diagnostics.json" \
      > "$out/$name-system/console.log" 2> "$out/$name-system/stderr.log"
    echo $? > "$out/$name-system.exit" );;
esac
case ",$routes," in *,slice,*)
  ( unset QCORE_NULL_PROPERTY
    export QCORE_INDIRECT_CENSUS="$out/$name-slice-census.jsonl"
    export QCORE_TYPED_PATH_PARITY="$out/$name-slice-parity.jsonl"
    export QCORE_INDIRECT_CENSUS_RECORDS="$out/$name-slice-records.jsonl"
    flock "$lock" bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 "fblb-$name" \
      "$binary" \
      --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 1 \
      > "$out/$name-slice.log" 2> "$out/$name-slice.stderr"
    echo $? > "$out/$name-slice.exit" );;
esac
