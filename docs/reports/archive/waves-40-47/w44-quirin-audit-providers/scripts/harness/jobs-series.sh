#!/bin/sh
# Jobs identity on the real corpus: `current` (full and contracts-only) at
# jobs 1 against the jobs-16 runs, and the plain route at jobs 1 and 16, plus
# the plain route against the base commit's binary.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/current-revision
cd "$root" || exit 2
# The base commit's quirin-audit, built from an archive of the base tree.
rm -rf private/base-src
mkdir -p private/base-src
git archive a2a63ced | tar -x -C private/base-src
(cd private/base-src && CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$root/private/base-target" \
  cargo build --release -p qcore-audit --bin quirin-audit > "$root/private/base-build.log" 2>&1)
echo "base build exit=$?"
cp private/base-target/release/quirin-audit private/base/base-quirin-audit
sha256sum private/base/base-quirin-audit
sh private/plain-run.sh plain1 1
# The plain route with the profile's manifest (no providers), branch and base.
for name in plain-manifest16 plain-manifest-base; do
  mkdir -p "private/runs/$name"
  python3 -c "
import json
print(json.dumps({'schema_version': 1,
  'profile_directory': '$root/contract/profiles/fluent-bit',
  'checkout': '$build/fluent-bit-clang-source',
  'compile_database': '$build/fluent-bit-clang-production/compile_commands.json',
  'libraries': ['cmetrics', 'ctraces', 'librdkafka', 'chunkio'],
  'report': '$root/private/runs/$name/report.json',
  'source_revision_policy': 'checkout_head'}, indent=2))
" > "private/runs/$name/config.json"
done
env -u QCORE_AUDIT_JOBS QCORE_CLANG=/usr/bin/clang "$root/private/bin/quirin-audit" --jobs 16 \
  private/runs/plain-manifest16/config.json > private/runs/plain-manifest16/audit.log 2>&1
echo "plain-manifest16 exit=$?"
env -u QCORE_AUDIT_JOBS QCORE_CLANG=/usr/bin/clang "$root/private/base/base-quirin-audit" \
  private/runs/plain-manifest-base/config.json > private/runs/plain-manifest-base/audit.log 2>&1
echo "plain-manifest-base exit=$?"
private/run-current.sh providers-config-j1 1 "$root/private/bin/quirin-audit" \
  --config "$root/private/identity/providers-config.json"
echo "providers-config-j1 exit=$?"
private/run-current.sh contracts-only-j1 1 "$root/private/bin/quirin-audit" \
  --config "$root/private/identity/providers-config.json" --contracts-only
echo "contracts-only-j1 exit=$?"
echo "jobs series done"
