#!/usr/bin/env bash
# gate-resume.sh NAME -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: run gate.sh's steps 05-09 into
# private/fbn/gate-NAME at the current HEAD (recorded in resume-head), after
# gate.sh NAME was stopped at step 05 for the disk floor (g5: free space fell
# to 11 GB while other cards built; this worktree's dev profile, 58.5 GiB,
# was cleaned first). Same commands, lock and file names as gate.sh.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
out="$repo/private/fbn/gate-$1"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  local label="$1"; shift
  flock "$lock" "$@" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
}
git rev-parse HEAD > "$out/resume-head"
step 05-probe cargo run -p qcore-compiler --example reevaluation_probe
step 06-followup cargo run -p qcore-compiler --example reevaluation_followup
step 07-lean python3 scripts/check_lean_audits.py
clang --version > "$out/clang-version.txt" 2>&1
echo $? > "$out/clang-version.exit"
echo done > "$out/code-done"
git rev-parse HEAD > "$out/docs-head"
step 08-python python3 -m unittest discover -s scripts/tests
step 09-index python3 scripts/build_docs_index.py --check
echo done > "$out/docs-done"
