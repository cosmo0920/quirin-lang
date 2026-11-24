#!/usr/bin/env bash
# build_audit.sh LABEL...: release quirin-audit at each tip worktree, copied to
# private/repair/bin/quirin-audit-LABEL.
here=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a86a9bde776af1664
mkdir -p "$here/private/repair/bin"
for label in "$@"; do
  # One target directory per tip (see tip_tests.sh).
  export CARGO_TARGET_DIR="$here/private/tip-target-$label"
  (cd "$here/private/tips/$label" && cargo build --release -p qcore-audit --bin quirin-audit) \
    >"$here/private/repair/build-$label.log" 2>&1
  status=$?
  echo "$label build exit $status"
  [ $status -eq 0 ] && cp "$CARGO_TARGET_DIR/release/quirin-audit" "$here/private/repair/bin/quirin-audit-$label"
done
