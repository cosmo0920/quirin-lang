#!/usr/bin/env bash
# tip_tests.sh TIPDIR LABEL [TEST...]: run the failing test binaries at one tip.
set -u
tip="$1"; label="$2"; shift 2
here=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a86a9bde776af1664
out="$here/private/repair/tips-$label.log"
# One target directory per tip: cargo's path-independent metadata hash and
# mtime freshness reuse another worktree's artifacts in a shared one.
export CARGO_TARGET_DIR="$here/private/tip-target-$label"
tests=("$@")
if [ ${#tests[@]} -eq 0 ]; then
  tests=(access_path_positions typed_root_declarations lexical_closure_nested_assignment
         typed_const_pointer_value scalar_read_identity detach_lookup_membership)
fi
args=()
for t in "${tests[@]}"; do args+=(--test "$t"); done
cd "$tip" || exit 2
cargo test -p qcore-c-gateway "${args[@]}" --no-fail-fast -- --test-threads=4 >"$out" 2>&1
status=$?
echo "exit $status" >>"$out"
exit $status
