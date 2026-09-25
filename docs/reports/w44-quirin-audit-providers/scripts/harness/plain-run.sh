#!/bin/sh
# usage: plain-run.sh NAME JOBS -- the plain `quirin-audit CONFIG.json` route over the
# prepared current-revision Clang build, with the Cycle 19 provider selection.
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/current-revision
cd "$root" || exit 2
name=$1
jobs=$2
mkdir -p "private/runs/$name"
python3 -c "
import json
entries = open('private/identity/providers.txt').read().split()
print(json.dumps({'schema_version': 1,
  'profile_directory': '$root/contract/profiles/fluent-bit',
  'checkout': '$build/fluent-bit-clang-source',
  'compile_database': '$build/fluent-bit-clang-production/compile_commands.json',
  'libraries': ['cmetrics', 'ctraces', 'librdkafka', 'chunkio'],
  'report': '$root/private/runs/$name/report.json',
  'source_revision_policy': 'checkout_head',
  'load_path': '$root/providers/contract-load-path.json',
  'providers': entries}, indent=2))
" > "private/runs/$name/config.json"
env -u QCORE_AUDIT_JOBS QCORE_CLANG=/usr/bin/clang /usr/bin/time -v -o "private/runs/$name/time.txt" \
  "$root/private/bin/quirin-audit" --jobs "$jobs" "private/runs/$name/config.json" > "private/runs/$name/audit.log" 2>&1
status=$?
echo "$status" > "private/runs/$name/exit.txt"
echo "$name exit=$status"
