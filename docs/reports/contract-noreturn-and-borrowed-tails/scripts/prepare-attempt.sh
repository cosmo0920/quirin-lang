#!/usr/bin/env bash
# Prepare a fresh private Clang build of Fluent Bit at 046accf6 for
# CONTRACT-NORETURN-AND-BORROWED-TAILS (w48-callee-closure-r3/scripts/prepare-attempt.sh),
# under the shared heavy lock. `prepare` clones the user's checkout without
# hardlinks and never writes to it.
# usage: prepare-attempt.sh ATTEMPT BINARY [REVISION]
set -u
attempt="$1"
binary="$2"
revision="${3:-046accf6aded7906f66a1baf0202da349e521736}"
out_root=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/contract-noreturn-and-borrowed-tails
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out_root"
flock "$lock" timeout 3600 "$binary" prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision "$revision" \
  --output "$out_root/$attempt" \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  > "$out_root/prepare-$attempt.log" 2>&1
echo $? > "$out_root/prepare-$attempt.exit"
cat "$out_root/prepare-$attempt.exit"
