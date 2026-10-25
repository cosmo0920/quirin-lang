#!/bin/sh
# MEASURE-TOOL: re-read two recorded sides through the committed live profiles
# (`--profile`, contract/evaluations/), so the record carries
# evaluation_profiles.py's figures and its compare verdict against each
# profile's baseline. No audit runs.
# Usage: profile_figures.sh WORKTREE OUT_DIR BASE_SIDE_DIR TIP_SIDE_DIR
set -u
W=$1
cd "$W" || exit 2
"$W/target/debug/quirin-audit" --plugin-path "$W/target/debug" --output "$2" measure \
  --profile fluent-bit-system --profile fluent-bit-tail-pause \
  --side base=run:"$3" --side tip=run:"$4" --repository . --no-slots >"$2.log" 2>&1
echo $? >"$2.exit"
