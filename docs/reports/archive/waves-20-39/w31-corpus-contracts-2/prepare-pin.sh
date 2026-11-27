#!/bin/sh
# Fresh Clang pin for card W31-CORPUS-CONTRACTS-2 (wave 31), per the
# established discipline: neither the long-lived developer database nor
# another card's pin is reused, and the pin's own production database is
# copied into <source>/build/compile_commands.json, which the preparation
# script does not create and which the pinned route reads.
set -eu
WT=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-afee9ca14eea086a7
PIN="$WT/build/cycle19/clang-pin"
cd "$WT"
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output "$PIN" \
  --clang /usr/bin/clang --clangxx /usr/bin/clang++
mkdir -p "$PIN/fluent-bit-clang-source/build"
cp "$PIN/fluent-bit-clang-production/compile_commands.json" \
   "$PIN/fluent-bit-clang-source/build/compile_commands.json"
sha256sum "$PIN/fluent-bit-clang-source/build/compile_commands.json"
python3 -c "import json,sys;print(len(json.load(open(sys.argv[1]))),'entries')" \
   "$PIN/fluent-bit-clang-source/build/compile_commands.json"
echo PREPARE_OK
