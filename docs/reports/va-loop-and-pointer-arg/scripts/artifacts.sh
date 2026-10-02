#!/usr/bin/env bash
# artifacts.sh -- copy the card's measurement records from private/valoop into
# artifacts/ (comparisons, censuses, digests, probe rows).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/valoop"
art="$here/../artifacts"
probe=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/va-loop/probe/out
mkdir -p "$art" || exit 2
for f in compare-corpus.json compare-system.json compare-slice.json census-before-corpus.json \
  census-after-corpus.json census-before-slice.json census-after-slice.json \
  before-digests.txt after-digests.txt before.binary.sha256 after.binary.sha256; do
  cp "$out/$f" "$art/$f"
done
for f in before-tip before-master after-tip after-master; do
  python3 "$here/probe_rows.py" "$probe/$f.json" > "$art/probe-$f.jsonl"
done
ls -la "$art"
