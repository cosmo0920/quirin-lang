#!/usr/bin/env bash
# measure-base.sh REV NAME -- SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN: the same
# bounds-selected arbiter run at an earlier main revision REV, for attribution
# only. The tree at REV is exported (git archive, no checkout, no worktree)
# into private/sup03/src-NAME, its release quirin-audit built there under the
# shared heavy lock into its own target directory, and the arbiter run from
# that tree (so the contract load path is REV's). Output: private/sup03/NAME-*.
# Exits to files; nothing piped except git archive into tar, whose exits are
# both recorded (PIPESTATUS).
set -u
rev="$1"; name="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/sup03"
src="$out/src-$name"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$src" || exit 2
cd "$repo" || exit 2
git rev-parse "$rev^{commit}" > "$out/$name.rev" || exit 2
git archive "$rev" | tar -x -C "$src"
echo "${PIPESTATUS[*]}" > "$out/$name-export.exit"
# git archive leaves submodules empty; the arbiter verifies third_party/libevent's
# revision, so link this checkout's initialised submodule when REV pins the same
# commit (checked, not assumed). First attempt `base` lacked this and failed
# with SourceRevisionMismatch; it is kept, not used.
[ "$(git rev-parse "$rev:third_party/libevent")" = "$(git rev-parse HEAD:third_party/libevent)" ] || exit 4
rmdir "$src/third_party/libevent" 2>/dev/null
[ -e "$src/third_party/libevent" ] || ln -s "$repo/third_party/libevent" "$src/third_party/libevent"
cd "$src" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit > "$out/$name-build.log" 2>&1
echo $? > "$out/$name-build.exit"
[ "$(cat "$out/$name-build.exit")" = 0 ] || exit 1
sha256sum target/release/quirin-audit > "$out/$name-bounds.binary.sha256"
flock "$lock" /usr/bin/env bash -c '
  start=$EPOCHREALTIME
  QCORE_BOUNDS_PROPERTY="$1-bounds.jsonl" python3 scripts/run_pinned_audit.py "$1-bounds" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$1-bounds.log" 2> "$1-bounds.stderr"
  rc=$?
  end=$EPOCHREALTIME
  echo "$rc" > "$1-bounds.exit"
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$1-bounds.seconds"
' _ "$out/$name"
cat "$out/$name-bounds.exit"
