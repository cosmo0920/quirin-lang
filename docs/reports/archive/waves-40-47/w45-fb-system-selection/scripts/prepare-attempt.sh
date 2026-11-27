#!/usr/bin/env bash
# Prepare a fresh Clang build of Fluent Bit 864c3d97 for W45-FB-SYSTEM-SELECTION.
# usage: prepare-attempt.sh ATTEMPT BINARY
# ATTEMPT must be a fresh name; the build goes to OUT_ROOT/ATTEMPT, never into
# the user's checkout or build/cycle19. The prepare log and exit go beside it.
set -u
attempt="$1"
binary="$2"
out_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-system-selection
timeout 3600 "$binary" prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 864c3d97f3589515285a19396ed8dfc85cda08ae \
  --output "$out_root/$attempt" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  > "$out_root/prepare-$attempt.log" 2>&1
echo $? > "$out_root/prepare-$attempt.exit"
