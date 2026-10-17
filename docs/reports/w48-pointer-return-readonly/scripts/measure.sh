#!/usr/bin/env bash
# measure.sh build|corpus|system|slice RUN -- isolated W48 measurements.
# Each invocation uses a fresh private directory and records the unpiped exit.
set -u
mode="${1:?build or corpus}"
run="${2:?fresh run name}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/w48-pointer-return/$run"
if [ -e "$out" ]; then
  echo "run directory already exists: $out" >&2
  exit 2
fi
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
binary="$repo/target/release/quirin-audit"
case "$mode" in
  build)
    cargo build --release -p qcore-audit --bin quirin-audit -j2 \
      > "$out/build.log" 2>&1
    status=$?
    echo "$status" > "$out/build.exit"
    if [ "$status" -eq 0 ]; then
      sha256sum "$binary" > "$out/binary.sha256"
    fi
    exit "$status";;
  corpus)
    QCORE_TYPED_PATH_PARITY="$out/parity.jsonl" \
      python3 scripts/run_pinned_audit.py "$out/corpus" 66910c10 \
        --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
        --jobs 4 > "$out/console.log" 2>&1
    status=$?
    echo "$status" > "$out/corpus.exit"
    exit "$status";;
  system)
    prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
    QCORE_TYPED_PATH_PARITY="$out/parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
      "$binary" current \
      --checkout "$prepared/fluent-bit-clang-source" \
      --profile-directory "$repo/contract/profiles/fluent-bit" \
      --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
      --selection "$repo/contract/profiles/fluent-bit/system-selection.json" \
      --jobs 4 --report "$out/resource-diagnostics.json" \
      > "$out/console.log" 2> "$out/stderr.log"
    status=$?
    echo "$status" > "$out/system.exit"
    exit "$status";;
  slice)
    prepared=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
    QCORE_TYPED_PATH_PARITY="$out/parity.jsonl" QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
      timeout 3600 "$binary" current \
      --checkout "$prepared/fluent-bit-clang-source" \
      --profile-directory "$repo/contract/profiles/fluent-bit" \
      --compile-database "$prepared/fluent-bit-clang-production/compile_commands.json" \
      --selection "$repo/contract/profiles/fluent-bit/tail-pause-selection.json" \
      --jobs 4 --report "$out/resource-diagnostics.json" \
      > "$out/console.log" 2> "$out/stderr.log"
    status=$?
    echo "$status" > "$out/slice.exit"
    exit "$status";;
  *)
    echo "unknown mode: $mode" >&2
    exit 2;;
esac
