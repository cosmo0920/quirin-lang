#!/usr/bin/env bash
# rerun-fmt.sh NAME -- REFCOUNT-RESOURCE-KIND: after gate NAME's first attempt
# found `cargo fmt --check` red on one test file (a whitespace-only change,
# fixed by its own commit), keep the first attempt's fmt logs and the
# stale-matrix Python run under *-attempt1 / *-stale-matrix names, and rerun at
# the fixed head: the standing and provider fmt checks, `cargo check` and
# clippy. Each step under the shared heavy lock; exits to files.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
gate="$repo/private/rrrd/gate-$name"
provider="$repo/private/rrrd/provider-gate-$name"
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
keep() { [ -f "$1.exit" ] && mv "$1.log" "$1-$2.log" && mv "$1.exit" "$1-$2.exitcode"; }
keep "$gate/02-fmt" attempt1
keep "$provider/p3-fmt" attempt1
keep "$gate/08-python" stale-matrix
git rev-parse HEAD > "$gate/rerun-head"
step() {
  local file="$1"; shift
  flock "$lock" "$@" > "$file.log" 2>&1
  echo $? > "$file.exit"
  echo "$(basename "$file") $(cat "$file.exit")"
}
step "$gate/02-fmt" cargo fmt --all -- --check
step "$provider/p3-fmt" cargo fmt --manifest-path providers/Cargo.toml --all -- --check
step "$gate/00-check-rerun" cargo check --workspace --all-targets
step "$gate/03-clippy-rerun" cargo clippy --locked --all-targets -- -D warnings
step "$gate/08-python" python3 -m unittest discover -s scripts/tests
