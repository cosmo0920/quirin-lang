#!/bin/bash
# LEAN-4-35-TOOLCHAIN inventory: run the three standalone Lean files under both
# toolchains (selected per file with elan's `+toolchain`, outside proof/, so
# neither the pin nor the elan default is touched) and record the outputs in
# artifacts/inventory/. Light: three single-file elaborations per version.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/../artifacts/inventory"
mkdir -p "$out"
for v in v4.34.0 v4.35.0; do
  for f in token_table reserved_names beq_axioms; do
    "$HOME/.elan/bin/lean" "+leanprover/lean4:$v" "$here/$f.lean" > "$out/$f-$v.txt" 2>&1
    echo $? > "$out/$f-$v.exit"
  done
done
# Paths in error lines name this worktree; keep only the file name.
sed -i -E 's#^.*/(reserved_names\.lean)#\1#' "$out"/reserved_names-*.txt
diff "$out/token_table-v4.34.0.txt" "$out/token_table-v4.35.0.txt" > "$out/token_table.diff"
echo $? > "$out/token_table.diff.exit"
diff "$out/beq_axioms-v4.34.0.txt" "$out/beq_axioms-v4.35.0.txt" > "$out/beq_axioms.diff"
echo $? > "$out/beq_axioms.diff.exit"
grep -H . "$out"/*.exit
