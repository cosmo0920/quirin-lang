#!/bin/sh
# corpus.sh NAME -- CG23-RELATIONAL-POSTCONDITIONS: build this worktree's
# release quirin-audit and run only the corpus arbiter into
# private/cg23rp/NAME (measure.sh's first two steps). Run under the shared
# heavy-work flock. Exits captured to files; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg23rp"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit \
  > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
sha256sum "$repo/target/release/quirin-audit" > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" python3 scripts/run_pinned_audit.py \
  "$out/$name" 66910c10 --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 \
  --jobs 4 > "$out/$name.log" 2>&1
echo $? > "$out/$name.exit"
cat "$out/$name.exit"
