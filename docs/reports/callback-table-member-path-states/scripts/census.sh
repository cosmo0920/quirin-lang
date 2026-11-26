#!/bin/sh
# census.sh -- the shipped table-clause census: every shipped .qv naming
# resource_callback_table, and every member-path state (`|->`) in the
# cmetrics and ctraces providers. Run from the repository root.
echo "# grep -rl --include='*.qv' 'resource_callback_table(' providers contract"
grep -rl --include='*.qv' 'resource_callback_table(' providers contract | sort
echo "# grep -rc --include='*.qv' '|->' providers/qcore-provider-cmetrics providers/qcore-provider-ctraces | grep -v ':0\$'"
grep -rc --include='*.qv' '|->' providers/qcore-provider-cmetrics providers/qcore-provider-ctraces | grep -v ':0$'
echo "# grep exit $? (1: no member-path state in either provider)"
