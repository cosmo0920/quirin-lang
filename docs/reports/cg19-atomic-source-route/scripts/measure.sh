#!/bin/sh
# measure.sh PHASE -- CG19-ATOMIC-SOURCE-ROUTE's corpus arbiter with this
# worktree's release quirin-audit, into a fresh private/cg19/PHASE directory.
# SKIP_BUILD=1 measures the release binary already built. Every exit is
# written to a file; nothing is piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
phase="$1"
out="$repo/private/cg19/$phase"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
if [ "${SKIP_BUILD:-0}" != 1 ]; then
  CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit \
    > "$out/build.log" 2>&1
  echo $? > "$out/build.exit"
fi
sha256sum "$repo/target/release/quirin-audit" > "$out/binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/arbiter/parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/arbiter" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/arbiter.log" 2>&1
echo $? > "$out/arbiter.exit"
