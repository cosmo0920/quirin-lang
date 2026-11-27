#!/bin/sh
# sel.sh OUT BINARY TREE SELECTION [streams] : a unit-scoped current run (round 3's run_current_census.py)
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aab78fbbfefedf82b
OUT=$1; BIN=$2; TREE=$(cd "$3" && pwd); SEL=$4
mkdir -p "$OUT.args"
python3 $W/docs/reports/openssl-provider-plugin/evidence/opt_in_providers.py "$TREE" > "$OUT.args/providers.args"
export R3_PREPARED=$W/private/r4/fb-046accf6
export R3_TREE=$TREE
export R3_PROVIDERS=$OUT.args/providers.args
[ "$5" = streams ] || export R3_REPORT_ONLY=1
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh python3 $W/docs/reports/cg10-indirect-access-sole-residuals-r3/scripts/run_current_census.py "$OUT" "$BIN" "$SEL" 8 > "$OUT.args/run.log" 2>&1
echo $? > "$OUT.args/exit"
cat "$OUT.args/exit"
