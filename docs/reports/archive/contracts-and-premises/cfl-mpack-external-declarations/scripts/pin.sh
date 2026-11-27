#!/usr/bin/env bash
# CFL-MPACK-EXTERNAL-DECLARATIONS: pin the reviewed declarations at the three
# audited revisions, then check every crate's lock at each. Run from the
# repository root; each step's output and exit go to LOG_DIR.
# usage: pin.sh BINARY LOG_DIR ROOT_66910c10 ROOT_046accf6 ROOT_34155955
set -u
binary="$1"
logs="$2"
shift 2
roots=("$@")
here="$(cd "$(dirname "$0")" && pwd)"
declared="$here/../census/declared.txt"
mkdir -p "$logs"
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
functions() {
  grep -v '^#' "$declared" | grep "^$1" | sed 's/^/--function /' | tr '\n' ' '
}
tags=(66910c10 046accf6 34155955)
for i in 0 1 2; do
  # shellcheck disable=SC2046
  step "pin-cfl-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate quirin-provider-cfl $(functions cfl_)
  # shellcheck disable=SC2046
  step "pin-mpack-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --crate quirin-provider-mpack $(functions mpack_)
done
for i in 0 1 2; do
  step "check-${tags[$i]}" "$binary" pin --source-root "${roots[$i]}" --check
done
