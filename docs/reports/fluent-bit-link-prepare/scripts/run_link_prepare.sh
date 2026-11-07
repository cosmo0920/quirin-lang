#!/bin/bash
# The card's prepare run: `quirin-audit prepare clang-build --link` of Fluent
# Bit 046accf6 into a fresh private output, under an audit gate slot (the
# build is CPU-bound; 4 GiB admission). Usage: run_link_prepare.sh WORKTREE RUN
# [LABEL]. A second invocation with the same RUN reuses that prepared output
# (prepare configures it again, rebuilds what changed and links again); LABEL
# names its receipts (default RUN).
set -euo pipefail
worktree=$1
run=$2
label=${3:-$run}
out="$worktree/private/link-prepare/$run"
mkdir -p "$out"
cd "$worktree"
exec python3 scripts/gate_slot.py audit --memory-budget 4G \
  --receipt "$worktree/private/link-prepare/$label.receipt.json" \
  --wait-record "$worktree/private/link-prepare/$label.wait.json" -- \
  /usr/bin/time -v "$worktree/target/release/quirin-audit" prepare clang-build \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 046accf6aded7906f66a1baf0202da349e521736 \
  --output "$out" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  --repository "$worktree" \
  --link --build-jobs 6
