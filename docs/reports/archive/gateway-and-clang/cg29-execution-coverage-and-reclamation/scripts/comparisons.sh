#!/usr/bin/env bash
# comparisons.sh -- CG29's BEFORE (base f6336c9d) -> AFTER (tip on main
# 6cab1ad9) comparisons of the corpus, system and tail-pause slice runs, into
# artifacts/. The difference includes the landings between the base and main
# (CG27, CG21, CG30, CG16); README section 4 attributes it.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/cg29"
t=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
a="$here/../artifacts"
cd "$here" || exit 2
python3 compare.py "$p/before" "$p/after" --json "$a/corpus-comparison.json" > "$p/cmp-corpus.txt" 2>&1
echo $? > "$p/cmp-corpus.exit"
python3 compare.py "$p/before-system" "$p/after-system" --json "$a/system-comparison.json" > "$p/cmp-system.txt" 2>&1
echo $? > "$p/cmp-system.exit"
python3 compare.py "$t/cg29-before" "$t/cg29-after" --json "$a/slice-comparison.json" > "$p/cmp-slice.txt" 2>&1
echo $? > "$p/cmp-slice.exit"
