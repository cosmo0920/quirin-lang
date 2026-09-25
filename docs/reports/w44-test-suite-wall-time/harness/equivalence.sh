#!/bin/bash
# Usage (worktree root): equivalence.sh <base output.log> (pre:<other> | post:<other>)...
# Compares every <other> (a runner summary.json or a plain cargo output.log)
# with the base run. `pre:` runs predate the Clang resolution cache commit
# (585b4260) and must be identical. `post:` runs include it; its two unit tests
# are the only declared additions, and they must pass.
here="$(cd "$(dirname "$0")" && pwd)"
base="$1"
shift
status=0
for spec in "$@"; do
  kind="${spec%%:*}"
  other="${spec#*:}"
  echo "== $kind $other"
  if [ "$kind" = post ]; then
    python3 "$here/compare.py" "$base" "$other" \
      --expect-added "qcore_c_gateway::test clang::tests::a_failed_resolution_is_never_cached_or_masked ... ok" \
      --expect-added "qcore_c_gateway::test clang::tests::a_successful_resolution_is_reused_only_under_the_same_environment ... ok"
  else
    python3 "$here/compare.py" "$base" "$other"
  fi
  code=$?
  echo "exit $code"
  [ $code -ne 0 ] && status=1
done
exit $status
