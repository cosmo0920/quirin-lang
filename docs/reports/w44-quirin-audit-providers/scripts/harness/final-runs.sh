#!/bin/sh
# Release build of the final tree, the pinned arbiter, the confirmation
# identity series and the README examples, one after another.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab16a94e01ece061d
cd "$root" || exit 2
build=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/current-revision
CARGO_INCREMENTAL=0 cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit > private/final-build-example.log 2>&1
echo "build example exit=$?"
CARGO_INCREMENTAL=0 cargo build --release -p qcore-audit --bin quirin-audit > private/final-build-audit.log 2>&1
echo "build quirin-audit exit=$?"
cp target/release/examples/cycle19_fluent_bit_libraries_audit private/bin/example
cp target/release/quirin-audit private/bin/quirin-audit
sha256sum private/bin/example private/bin/quirin-audit
commit=$(git rev-parse HEAD)
python3 docs/reports/w43-return-expression/scripts/run_audit.py private/arbiter "$commit" --jobs 16 > private/arbiter.log 2>&1
echo "arbiter exit=$?"
python3 docs/reports/w44-branch-condition-calls/scripts/baseline_identity.py private/arbiter \
  docs/reports/w44-collection-allocation/integration/artifacts \
  --root-alias "$root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" \
  > private/identity/baseline-identity-arbiter.json 2>&1
echo "baseline_identity exit=$?"
private/series.sh 2 > private/series2.log 2>&1
echo "series2 exit=$?"
cat private/series2.log
# README examples, over the same prepared build.
mkdir -p private/runs/readme-cli private/runs/readme-json
"$root/private/bin/quirin-audit" current \
  --checkout "$build/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database "$build/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json \
  --load-path providers/contract-load-path.json \
  --provider quirin-provider-libc \
  --provider quirin-provider-fluent-bit:fluent_bit/resource_memory.qcore \
  --provider qcore-provider-chunkio:chunkio/chunk_residency.qcore \
  --monkey-source-report private/runs/readme-cli/monkey-source.json \
  --report private/runs/readme-cli/resource-diagnostics.json \
  --jobs 16 > private/runs/readme-cli/audit.log 2>&1
echo "readme-cli exit=$?"
python3 -c "
import json
c = json.load(open('private/identity/providers-config.json'))
print(json.dumps({'schema_version': 1,
  'checkout': '$build/fluent-bit-clang-source',
  'profile_directory': '$root/contract/profiles/fluent-bit',
  'compile_database': '$build/fluent-bit-clang-production/compile_commands.json',
  'load_path': '$root/providers/contract-load-path.json',
  'providers': ['quirin-provider-libc', 'quirin-provider-mpack', 'quirin-provider-msgpack'],
  'mode': 'contracts-only', 'quirin_providers_only': True, 'jobs': 16,
  'report': '$root/private/runs/readme-json/contracts-only.json'}, indent=2))
" > private/identity/readme-contracts-only.json
"$root/private/bin/quirin-audit" current --config private/identity/readme-contracts-only.json \
  > private/runs/readme-json/audit.log 2>&1
echo "readme-json exit=$?"
echo "final runs done"
