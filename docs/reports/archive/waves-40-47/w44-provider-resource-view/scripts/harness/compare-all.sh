#!/bin/sh
# usage: compare-all.sh SUFFIX -- every identity comparison of one series,
# written to docs/reports/w44-provider-resource-view/identity/.
set -u
root=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-aa6cef7bd1d8bab9c
s=$1
cd "$root" || exit 2
scripts=docs/reports/w44-provider-resource-view/scripts
old=docs/reports/w44-quirin-audit-providers/scripts
out=docs/reports/w44-provider-resource-view/identity
runs=private/runs
mkdir -p "$out"
sel=resource-diagnostics.contracts.json:provider_selection
ev1=resource-diagnostics.json:evidence
ev2=resource-diagnostics.contracts.json:evidence
compare() {
  name=$1
  shift
  python3 "$old/compare_outputs.py" "$@" > "$out/compare-$name$s.json" 2>&1
  echo "$name$s exit=$?"
}
# Full mode is unchanged: the base binary's manifest run against the branch's.
compare base-vs-manifest "$runs/base$s" "$runs/manifest$s"
# All eleven crates whole against the same 47 entries in whole-crate order.
compare canonical-vs-whole "$runs/canonical$s" "$runs/whole$s" --added "$sel"
# The contracts-only default against the same manifest, contracts-only.
compare canonical-co-vs-all-loaded "$runs/canonical-co$s" "$runs/all-loaded$s" --added "$sel"
# Against the committed Cycle 19 manifest: only its order-bound digests differ.
compare manifest-vs-whole "$runs/manifest$s" "$runs/whole$s" --added "$sel"
compare manifest-co-vs-all-loaded "$runs/manifest-co$s" "$runs/all-loaded$s" --added "$sel"
compare manifest-vs-canonical "$runs/manifest$s" "$runs/canonical$s"
python3 "$old/leaf_buckets.py" "$runs/manifest$s/resource-diagnostics.json" \
  "$runs/whole$s/resource-diagnostics.json" > "$out/leaf-buckets-manifest-vs-whole$s.json"
echo "leaf-buckets$s exit=$?"
# The whole run recorded exactly the canonical manifest as its contract_manifest.
python3 -c "
import json, sys
whole = json.load(open('$runs/whole$s/resource-diagnostics.contracts.json'))
allr = json.load(open('$runs/all-loaded$s/resource-diagnostics.contracts.json'))
text = open('cycle19-canonical-order.json').read()
ok = whole['contract_manifest'] == text and allr['contract_manifest'] == text
print(json.dumps({'whole_contract_manifest_is_canonical': whole['contract_manifest'] == text,
                  'all_loaded_contract_manifest_is_canonical': allr['contract_manifest'] == text,
                  'all_loaded_selection': allr['provider_selection']['selection'],
                  'all_loaded_modules': len(allr['provider_selection']['modules'])}, indent=1))
sys.exit(0 if ok else 1)
" > "$out/lowered-is-canonical$s.json"
echo "lowered-is-canonical$s exit=$?"
for run in manifest whole canonical all-loaded canonical-co manifest-co base; do
  [ -d "$runs/$run$s" ] || continue
  printf '%s %s %s\n' "$run$s" "$(sha256sum "$runs/$run$s/resource-diagnostics.json" | cut -d' ' -f1)" \
    "$(tail -2 "$runs/$run$s/audit.log" | head -1)"
done > "$out/run-digests$s.txt"
cp "$runs/whole$s/binary.sha256" "$out/binary$s.sha256"
cp "$runs/base$s/binary.sha256" "$out/binary-base$s.sha256" 2>/dev/null || true
