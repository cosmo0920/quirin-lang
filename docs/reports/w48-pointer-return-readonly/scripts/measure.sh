#!/usr/bin/env bash
# measure.sh build|corpus RUN -- isolated W48 release build and pinned corpus.
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
case "$mode" in
  build)
    cargo build --release -p qcore-audit --bin quirin-audit -j2 \
      > "$out/build.log" 2>&1
    status=$?
    echo "$status" > "$out/build.exit"
    if [ "$status" -eq 0 ]; then
      sha256sum target/release/quirin-audit > "$out/binary.sha256"
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
  *)
    echo "unknown mode: $mode" >&2
    exit 2;;
esac
