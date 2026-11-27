#!/bin/sh
# W44-CONTRACTS-ONLY-NO-BUILD measurement on the read-only Fluent Bit checkout.
# usage: measure.sh RUN [root]
#   RUN   fresh run name; outputs in the worktree-private private/measure/RUN
#   root  also run the simplest form (the whole checkout, no flags)
# Every exit is captured to a file and nothing is piped.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa974c0228e7d5c45
prep=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-hungry-thompson-1740da/4e877459-f647-4ada-9799-501505fa7da8/scratchpad/prep
fb=/home/cosmo/Documents/GitHub/fluent-bit
cd "$root" || exit 2
run="$root/private/measure/${1:?usage: measure.sh RUN [root]}"
if [ -e "$run" ]; then
  echo "$run exists" >&2
  exit 2
fi
mkdir -p "$run"
bin="$root/target/release/quirin-audit"
QCORE_CLANG=$(command -v clang)
export QCORE_CLANG
sha256sum "$bin" > "$run/binary.sha256"
step() {
  name=$1
  shift
  date +%s > "$run/$name.start"
  "$@" > "$run/$name.log" 2>&1
  echo $? > "$run/$name.exit"
  date +%s > "$run/$name.end"
  echo "$name exit=$(cat "$run/$name.exit")"
}
step selection python3 docs/reports/w44-contracts-only-no-build/scripts/selection.py \
  "$fb" contract/profiles/fluent-bit "$prep" "$root/providers/contract-load-path.json" "$run"
step configured "$bin" current --checkout "$prep/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database "$prep/fluent-bit-clang-production/compile_commands.json" \
  --load-path providers/contract-load-path.json --contracts-only \
  --report "$run/configured/contracts-only.json" --jobs 16
step bare "$bin" contracts --config "$run/bare.json"
step src-includes "$bin" contracts --config "$run/src-includes.json"
if [ "${2:-}" = root ]; then
  step root "$bin" contracts --source-root "$fb" --report "$run/root/report.json" --jobs 16
fi
echo "measure done"
