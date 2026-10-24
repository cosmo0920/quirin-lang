#!/usr/bin/env bash
# The post-merge BASE/TIP pair in one hold of the shared lock: both release
# builds one after the other, then both pinned corpus arbiters at once
# (--jobs 4 each, typed-path parity), each into its own fresh directory.
# usage: pair-both.sh BASE_NAME BASE_ROOT TIP_NAME TIP_ROOT
set -u
here=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af8b2df032da9c24c/private/rkcf
flock /tmp/qcore-c-card-validation-20261022.lock /usr/bin/env bash -c '
  here="$1"; shift
  df -h / > "$here/pair.df-start"
  prep() {
    name="$1" root="$2"
    cd "$root" || return 2
    git rev-parse HEAD > "$here/$name.head"
    git submodule update --init --recursive > "$here/$name-submodules.log" 2>&1
    echo $? > "$here/$name-submodules.exit"
    CARGO_BUILD_JOBS=6 cargo build --release -p qcore-audit --bin quirin-audit > "$here/$name-build.log" 2>&1
    echo $? > "$here/$name-build.exit"
    sha256sum target/release/quirin-audit > "$here/$name.binary.sha256"
  }
  run() {
    name="$1" root="$2"
    cd "$root" || return 2
    start=$EPOCHREALTIME
    QCORE_TYPED_PATH_PARITY="$here/$name-parity.jsonl" python3 scripts/run_pinned_audit.py "$here/$name" 66910c10 \
      --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
      > "$here/$name.log" 2> "$here/$name.stderr"
    rc=$?
    end=$EPOCHREALTIME
    echo "$rc" > "$here/$name.exit"
    awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$here/$name.seconds"
  }
  prep "$1" "$2"
  prep "$3" "$4"
  run "$1" "$2" &
  run "$3" "$4" &
  wait
  df -h / > "$here/pair.df-end"
' _ "$here" "$@"
