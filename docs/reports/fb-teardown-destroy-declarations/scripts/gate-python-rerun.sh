#!/usr/bin/env bash
# gate-python-rerun.sh NAME -- FB-TEARDOWN-DESTROY-DECLARATIONS: gate NAME's
# code steps ran at HEAD; the only failures were two Python count pins
# (steps 08 and 13). The fix changes two Python test files and nothing else
# (`git diff --stat` recorded below), so the Rust, Lean and provider-Rust
# steps stand. This script moves the failed 13-provider-python and the docs
# part's logs into superseded/, records the diff from the gate's head, and
# reruns step 13 and the docs part (08, 09) at the new head, under the heavy
# lock. Every exit to a file; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/ftd/gate-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
mkdir -p "$out/superseded" || exit 2
for f in 13-provider-python 08-python 09-index; do
  mv "$out/$f.exit" "$out/$f.log" "$out/superseded/" 2>/dev/null
done
mv "$out/docs-head" "$out/docs-done" "$out/superseded/" 2>/dev/null
git diff --stat "$(cat "$out/head")" HEAD > "$out/python-rerun-diff.txt"
git rev-parse HEAD > "$out/python-rerun-head"
flock "$lock" python3 -m unittest discover -s providers/tests > "$out/13-provider-python.log" 2>&1
echo $? > "$out/13-provider-python.exit"
bash "$here/gate.sh" "$name" docs
grep -H . "$out"/13-provider-python.exit "$out"/08-python.exit "$out"/09-index.exit
