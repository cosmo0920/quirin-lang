#!/usr/bin/env bash
# branch_contracts_probe.sh PROBE_TREE PHASE BINARY -- the observed instance,
# measured without merging code. PROBE_TREE is an export of this card's tip
# (outside qcore-c/) into which overlay_branch_contracts.sh wrote the four
# contract-data files of W47-LOOP-OWNER-INSTANCE-FLOW (branch
# worktree-agent-aa0821f6bf47bd9c0): its input_instance.qv, the fluent-bit
# provider's contracts.lock, providers/contract-load-path.json and
# cycle19-resource-contracts.json. No code of that branch is used (its code
# changes are census-only, which the per-unit routes never run). BINARY is the
# base (fefb9199) release quirin-audit for BEFORE, this card's for AFTER. Runs
# the tail-pause slice exactly as run-current.sh does, into this worktree's
# private/rh/if-PHASE. Exit captured.
set -u
tree="$(cd "$1" && pwd -P)"
phase="$2"
binary="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rh/if-$phase"
prep=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
mkdir "$out" || exit 2
sha256sum "$binary" > "$out/binary.sha256"
cd "$tree" || exit 2
QCORE_CLANG=/usr/lib/llvm-21/bin/clang timeout 3600 "$binary" current \
  --checkout "$prep/fluent-bit-clang-source" \
  --profile-directory "$tree/contract/profiles/fluent-bit" \
  --compile-database "$prep/fluent-bit-clang-production/compile_commands.json" \
  --report "$out/resource-diagnostics.json" \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8 \
  > "$out/console.log" 2> "$out/stderr.log"
echo $? > "$out/exit"
