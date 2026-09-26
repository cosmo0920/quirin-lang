#!/usr/bin/env bash
# Rerun the record-dependent gate steps after the shared records are edited.
# usage: validate_records.sh LOG_DIR
set -u
log="$1"
mkdir -p "$log"
step() {
  local name="$1"; shift
  "$@" > "$log/$name.log" 2>&1
  echo $? > "$log/$name.exit"
}
step python python3 -m unittest discover -s scripts/tests
step docs-index python3 scripts/build_docs_index.py --check
step diff-check git diff --check HEAD
