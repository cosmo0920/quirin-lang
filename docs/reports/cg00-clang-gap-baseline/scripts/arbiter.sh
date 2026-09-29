#!/bin/sh
# CG00-CLANG-GAP-BASELINE: one corpus arbiter run with the default release build.
# usage: arbiter.sh OUT_DIR   (relative to the checkout root; must not exist)
# Writes OUT_DIR/ plus OUT_DIR.{binary.sha256,run.log,exit,identity.txt,identity.exit}.
set -u
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
out="$root/$1"
cd "$root" || exit 2
mkdir -p "$(dirname "$out")"
sha256sum target/release/quirin-audit > "$out.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/parity.jsonl" python3 scripts/run_pinned_audit.py "$out" 66910c10 \
  --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 8 > "$out.run.log" 2>&1
echo $? > "$out.exit"
python3 docs/reports/cg00-clang-gap-baseline/scripts/identity.py "$out" \
  docs/reports/w47-flb-lib-lifecycle/integration/artifacts/corpus > "$out.identity.txt" 2>&1
echo $? > "$out.identity.exit"
