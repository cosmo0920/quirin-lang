#!/bin/bash
# Usage (worktree root): equivalence.sh <base output.log> <other>...
# Compares every <other> (a runner summary.json or a plain cargo output.log)
# with the base run. The two tests added by the Clang resolution cache commit
# (585b4260) are the only declared additions; they must pass.
here="$(cd "$(dirname "$0")" && pwd)"
base="$1"
shift
status=0
for other in "$@"; do
  echo "== $other"
  python3 "$here/compare.py" "$base" "$other" \
    --expect-added "qcore_c_gateway::test clang::tests::a_failed_resolution_is_never_cached_or_masked ... ok" \
    --expect-added "qcore_c_gateway::test clang::tests::a_successful_resolution_is_reused_only_under_the_same_environment ... ok"
  code=$?
  echo "exit $code"
  [ $code -ne 0 ] && status=1
done
exit $status
