#!/usr/bin/env bash
# Prepare (or re-verify) a private Clang build of Fluent Bit for W45-FB-ENGINE-PROGRESS.
# usage: prepare-attempt.sh ATTEMPT BINARY [REVISION] [LOG-SUFFIX]
# `quirin-audit prepare` clones the user's checkout with --no-hardlinks, detaches at
# REVISION, and refuses an existing source whose HEAD differs or that has tracked
# changes. Re-running it over an existing attempt (with a LOG-SUFFIX) is therefore
# the clean-status check after the native build.
# Output goes to OUT_ROOT/ATTEMPT, never into the user's checkout or build/cycle19.
set -u
attempt="$1"; binary="$2"
revision="${3:-046accf6aded7906f66a1baf0202da349e521736}"
suffix="${4:-}"
out_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress
timeout 3600 "$binary" prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision "$revision" \
  --output "$out_root/$attempt" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  > "$out_root/prepare-$attempt$suffix.log" 2>&1
echo $? > "$out_root/prepare-$attempt$suffix.exit"
cat "$out_root/prepare-$attempt$suffix.exit"
