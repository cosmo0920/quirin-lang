#!/bin/sh
# measure.sh NAME [system|current] -- the routes from private/fclc/side-NAME's
# own tree and release binary, each through an audit slot
# (scripts/gate_slot.py): the fluent-bit-system route (system.sh,
# FB-RETRY-OR6-ELEMENT-IDENTITY's, verbatim) and opt-in current
# --monkey-linked (opt-in-current.sh, FB-BACKLOG-ACCOUNTING's, with the
# archived opt_in_providers.py path). Both when no route is named. Outputs go
# to private/fclc/run-NAME-{system,current}.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a273dcc45e6a8fab7
S=$W/docs/reports/fb-chunk-lifecycle-contracts/scripts
T=$W/private/fclc/side-$1
P=/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
cd "$W" || exit 2
if [ "${2:-system}" = system ]; then
  python3 scripts/gate_slot.py audit --receipt "$W/private/fclc/$1-system.receipt.json" -- \
    sh "$S/system.sh" "$T" "$W/private/fclc/run-$1-system"
fi
if [ "${2:-current}" = current ]; then
  python3 scripts/gate_slot.py audit --receipt "$W/private/fclc/$1-current.receipt.json" -- \
    sh "$S/opt-in-current.sh" "$T" "$W/private/fclc/run-$1-current" "$T/target/release/quirin-audit" "$P"
fi
