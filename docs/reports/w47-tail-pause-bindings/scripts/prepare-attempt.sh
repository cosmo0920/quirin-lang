#!/usr/bin/env bash
# Prepare the W47-TAIL-PAUSE-BINDINGS slice inputs with the recorded
# `quirin-audit prepare` route (docs/reports/w45-fb-backlog-custody §4).
# usage: prepare-attempt.sh ATTEMPT BINARY SOURCE_REPO REVISION
#   SOURCE_REPO is a Git repository built from `git archive` of the Fluent Bit
#   commit (never the user's checkout); REVISION is its single commit, whose
#   tree must equal the archived commit's tree (checked by the caller).
# The output goes to OUT_ROOT/ATTEMPT/prep; the log and exit go beside it.
set -u
attempt="$1"; binary="$2"; source_repo="$3"; revision="$4"
out_root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail
out="$out_root/$attempt"
mkdir -p "$out" || exit 2
timeout 3600 "$binary" prepare \
  --fluent-bit "$source_repo" \
  --revision "$revision" \
  --output "$out/prep" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  > "$out/prepare.log" 2>&1
echo $? > "$out/prepare.exit"
cat "$out/prepare.exit"
