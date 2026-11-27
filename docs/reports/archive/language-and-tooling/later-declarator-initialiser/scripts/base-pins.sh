#!/bin/bash
# Run the tip's changed and new test files against the base sources (the
# base side tree, a git archive of 2c60986b0), to show each pin fails there.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a8ca049d3c5896ee3
B=$W/private/ldi/side-base
O=$W/private/ldi/base-pins
mkdir -p "$O"
cd "$B" || exit 2
for t in later_declarator_initialiser nullable_lazy_constructor ag1_indirect_member_access chained_assignment lexical_closure_nested_assignment member_access_projection scalar_statement_forms; do
  cp "$W/crates/qcore-c-gateway/tests/$t.rs" "crates/qcore-c-gateway/tests/$t.rs"
done
for t in later_declarator_initialiser nullable_lazy_constructor ag1_indirect_member_access chained_assignment lexical_closure_nested_assignment member_access_projection scalar_statement_forms; do
  sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo test -p qcore-c-gateway --test "$t" > "$O/$t.log" 2>&1
  echo "== $t exit $?"
  grep -E "^test .*(ok|FAILED)$|test result" "$O/$t.log"
done
