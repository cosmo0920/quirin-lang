#!/bin/sh
# Probe: every owned module selected alone. `loads` = the run reached the
# (deliberately absent) compile database.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/current-revision
cd "$root" || exit 2
python3 -c "
import json
for c in json.load(open('providers/contract-load-path.json'))['crates']:
    for m in c['modules']:
        print(c['name'] + ':' + m)
" | while read -r entry; do
  out=$("$root/private/bin/quirin-audit" current \
    --checkout "$build/fluent-bit-clang-source" \
    --profile-directory contract/profiles/fluent-bit \
    --compile-database /nonexistent/compile_commands.json \
    --load-path providers/contract-load-path.json \
    --provider "$entry" 2>&1)
  case "$out" in
    *"No such file or directory"*) echo "loads $entry" ;;
    *) echo "refused $entry" ;;
  esac
done
