#!/usr/bin/env bash
# Prepare a fresh private Clang build of Fluent Bit for W45-CONTAINER-ROOT-REACH.
# usage: prepare.sh ATTEMPT BINARY [REVISION]
set -u
attempt="$1"
binary="$2"
revision="${3:-046accf6aded7906f66a1baf0202da349e521736}"
out_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-container-root-reach
mkdir -p "$out_root"
timeout 3600 "$binary" prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision "$revision" \
  --output "$out_root/$attempt" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  > "$out_root/prepare-$attempt.log" 2>&1
echo $? > "$out_root/prepare-$attempt.exit"
cat "$out_root/prepare-$attempt.exit"
