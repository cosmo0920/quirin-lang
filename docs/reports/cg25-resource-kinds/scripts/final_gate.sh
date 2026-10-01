#!/bin/sh
# CG25-RESOURCE-KINDS: the final heavy steps, each meant to run under the
# shared heavy-step lock (`flock LOCK final_gate.sh STEP LOG_DIR`) the
# coordinator imposed at the 40 GB disk floor. STEP is `provider-test` or
# `gate`. Every exit is captured to a file; nothing is piped. Build output is
# removed after each step so the worktree stays small.
set -u
step="$1"; logs_rel="$2"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
cd "$root" || exit 2
logs="$root/$logs_rel"
mkdir -p "$logs"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
df -h / > "$logs/$step-df-before.txt" 2>&1
case "$step" in
  provider-test)
    cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline \
      > "$logs/p2-test.log" 2>&1
    echo $? > "$logs/p2-test.exit"
    cargo clean --manifest-path providers/Cargo.toml > "$logs/p2-clean.log" 2>&1
    ;;
  gate)
    sh "$here/gate.sh" "$logs_rel"
    cargo clean --profile dev > "$logs/clean.log" 2>&1
    ;;
esac
df -h / > "$logs/$step-df-after.txt" 2>&1
