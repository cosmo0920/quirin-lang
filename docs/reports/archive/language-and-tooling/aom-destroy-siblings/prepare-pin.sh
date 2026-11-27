#!/bin/sh
# Card AOM-DESTROY-SIBLINGS: prepare a FRESH Clang pin for the 185-unit audit.
# Wave-20 lesson: never reuse a developer-spelled compile database.
set -eu
WT=<worktree>
cd "$WT"
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output "$WT/build/cycle19/clang-pin" \
  --clang /usr/bin/clang --clangxx /usr/bin/clang++
