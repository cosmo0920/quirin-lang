#!/usr/bin/env bash
# measure-base-contracts.sh -- CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: the
# bounds-selected corpus with the BEFORE binary over the BASE contracts (the
# load path, chunkio inventory and bounds lock of 84ec54d1d, without
# chunkio/file_st_bounds.qv). The routes read the load path from the checkout at
# run time, so the BEFORE run (bin/before) already saw this card's three new
# declarations; this run isolates them. Under the shared heavy lock: the four
# provider files are written from the base revision's blobs, the run is made,
# and the files are restored from HEAD (round trip asserted by `git diff
# --exit-code`). Output: private/cg16-bounds-transport-and-chunkio-neighbours/base-bounds*.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg16-bounds-transport-and-chunkio-neighbours"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
flock "$lock" /usr/bin/env bash -c '
  set -u
  repo="$1"; out="$2"; base=84ec54d1d
  cd "$repo" || exit 2
  for f in providers/contract-load-path.json providers/qcore-provider-chunkio/contracts.json \
           providers/qcore-provider-chunkio/bounds-contracts.lock; do
    git show "$base:$f" > "$f" || exit 3
  done
  rm -f providers/qcore-provider-chunkio/contracts/chunkio/file_st_bounds.qv
  cat "$out/bin/before" > target/release/quirin-audit && chmod +x target/release/quirin-audit
  sha256sum target/release/quirin-audit > "$out/base-bounds.binary.sha256"
  start=$EPOCHREALTIME
  QCORE_BOUNDS_PROPERTY="$out/base-bounds.jsonl" python3 scripts/run_pinned_audit.py "$out/base-bounds" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$out/base-bounds.log" 2> "$out/base-bounds.stderr"
  echo $? > "$out/base-bounds.exit"
  end=$EPOCHREALTIME
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$out/base-bounds.seconds"
  git checkout HEAD -- providers/contract-load-path.json providers/qcore-provider-chunkio
  git diff --exit-code HEAD -- providers > "$out/base-bounds.restore.log" 2>&1
  echo $? > "$out/base-bounds.restore.exit"
' _ "$repo" "$out"
cat "$out/base-bounds.exit" "$out/base-bounds.restore.exit"
