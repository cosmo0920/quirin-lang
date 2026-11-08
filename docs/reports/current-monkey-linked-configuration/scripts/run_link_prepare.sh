#!/usr/bin/env bash
# usage: run_link_prepare.sh <binary> <outdir>
# This card's `quirin-audit prepare clang-build --link` of Fluent Bit 046accf6,
# run twice on one fresh private output (FLUENT-BIT-LINK-PREPARE's recipe: the
# second configure gives exactly the user's compile database). The tip binary
# keys the linked Monkey units as route units. Run under one audit slot.
set -u
BIN=$1
OUT=$2
W=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a97483da790d806af
mkdir -p "$OUT"
for pass in first second; do
  /usr/bin/time -v "$BIN" prepare clang-build \
    --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
    --revision 046accf6aded7906f66a1baf0202da349e521736 \
    --output "$OUT/run" \
    --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
    --repository "$W" \
    --link --build-jobs 6 > "$OUT/$pass.stdout.log" 2> "$OUT/$pass.stderr.log"
  echo $? > "$OUT/$pass.exit.txt"
  if [ -d "$OUT/run/link-resolution" ]; then
    rm -rf "$OUT/link-resolution.$pass"
    cp -r "$OUT/run/link-resolution" "$OUT/link-resolution.$pass"
  fi
done
