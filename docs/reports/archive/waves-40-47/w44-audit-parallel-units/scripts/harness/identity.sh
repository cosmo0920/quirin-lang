#!/bin/sh
# usage: identity.sh NAME  -> private/identity/NAME.json (+ exit), prints a one-line summary per artifact
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-af93c329e85457c34
name=$1
mkdir -p "$root/private/identity"
python3 "$root/docs/reports/w43-return-expression/scripts/baseline_identity.py" \
  "$root/private/runs/$name" "$root/docs/reports/w44-branch-condition-calls/integration/artifacts" \
  --root-alias "$root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" \
  > "$root/private/identity/$name.json"
status=$?
echo "$status" > "$root/private/identity/$name.exit"
python3 - "$root/private/identity/$name.json" "$status" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
print("exit", sys.argv[2], "verdict_bearing_identical", d["verdict_bearing_identical"])
for k, v in d["artifacts"].items():
    print(f"  {k}: raw_identical={v['byte_identical']} after_root_alias={v.get('identical_after_root_alias')}")
EOF
exit $status
