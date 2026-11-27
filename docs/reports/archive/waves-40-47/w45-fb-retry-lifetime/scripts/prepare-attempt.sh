#!/usr/bin/env bash
# Prepare a fresh Clang build of Fluent Bit (default 046accf6; REVISION overrides) for W45-FB-RETRY-LIFETIME.
# usage: prepare-attempt.sh ATTEMPT BINARY
# ATTEMPT must be a fresh name; the build goes to OUT_ROOT/ATTEMPT, never into
# the user's checkout or build/cycle19. The prepare log and exit go beside it.
set -u
attempt="$1"
binary="$2"
out_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-retry-lifetime
timeout 3600 "$binary" prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision "${REVISION:-046accf6aded7906f66a1baf0202da349e521736}" \
  --output "$out_root/$attempt" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  > "$out_root/prepare-$attempt.log" 2>&1
echo $? > "$out_root/prepare-$attempt.exit"
