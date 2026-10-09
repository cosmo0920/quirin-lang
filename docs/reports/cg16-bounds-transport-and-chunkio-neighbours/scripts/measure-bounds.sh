#!/usr/bin/env bash
# measure-bounds.sh NAME BINARY -- CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (adapted from QUANT-INDEX-CHECKS): the corpus arbiter with
# the bounds property selected (QCORE_BOUNDS_PROPERTY, card
# CG16-INDEX-AND-OFFSET-EXTENTS), the only audit route that runs the bounds
# consumer today (with an empty bounds environment), with a saved binary.
# run_pinned_audit.py always runs target/release/quirin-audit, so BINARY's
# bytes are written there first under the shared heavy lock and its digest
# recorded; run.json's binary_sha256 is the check that the run used them.
# Output: private/cg16-bounds-transport-and-chunkio-neighbours/NAME-bounds/ (the arbiter's directory) and
# private/cg16-bounds-transport-and-chunkio-neighbours/NAME-bounds.jsonl (the bounds rows). Exits to files; nothing piped.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg16-bounds-transport-and-chunkio-neighbours"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out" "$repo/target/release" || exit 2
cd "$repo" || exit 2
flock "$lock" /usr/bin/env bash -c '
  cat "$1" > "$2" && chmod +x "$2" && sha256sum "$2" > "$3" || exit 3
  start=$EPOCHREALTIME
  QCORE_BOUNDS_PROPERTY="$4-bounds.jsonl" python3 scripts/run_pinned_audit.py "$4-bounds" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$4-bounds.log" 2> "$4-bounds.stderr"
  rc=$?
  end=$EPOCHREALTIME
  echo "$rc" > "$4-bounds.exit"
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$4-bounds.seconds"
' _ "$binary" "$repo/target/release/quirin-audit" "$out/$name-bounds.binary.sha256" "$out/$name"
