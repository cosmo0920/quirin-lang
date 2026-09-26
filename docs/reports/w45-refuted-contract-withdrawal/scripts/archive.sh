#!/usr/bin/env bash
# Archive the card's measurements into the report (W45-REFUTED-CONTRACT-WITHDRAWAL).
# usage: archive.sh  (from the repository root, after the BEFORE and AFTER runs)
set -eu
report=docs/reports/w45-refuted-contract-withdrawal
out=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-refuted-contract-withdrawal/attempt-1
rw=private/rw
mkdir -p "$report/artifacts/slice" "$report/artifacts/arbiter"
# Corpus arbiter: identities against the baseline, and the sidecar delta.
cp "$rw/before-identity.json" "$report/artifacts/arbiter/before-identity.json"
cp "$rw/after-identity.json" "$report/artifacts/arbiter/after-identity.json"
python3 "$report/scripts/sidecar_delta.py" "$rw/arbiter-before/resource-diagnostics.contracts.json" \
  "$rw/arbiter-after/resource-diagnostics.contracts.json" > "$report/artifacts/arbiter/contracts-delta.json"
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); json.dump(d['refuted_contracts'], sys.stdout, indent=1); print()" \
  "$rw/arbiter-after/resource-diagnostics.contracts.json" > "$report/artifacts/arbiter/refuted-contracts-section.json"
cp "$rw/arbiter-before/run.json" "$report/artifacts/arbiter/before-run.json"
cp "$rw/arbiter-after/run.json" "$report/artifacts/arbiter/after-run.json"
# Slice: BEFORE and AFTER reports, their comparison, the section, and --jobs 1.
gzip -9 -n -c "$out/slice-before/resource-diagnostics.json" > "$report/artifacts/slice/before-046accf6.json.gz"
gzip -9 -n -c "$out/slice-after/resource-diagnostics.json" > "$report/artifacts/slice/after-046accf6.json.gz"
gzip -9 -n -c "$out/slice-after/resource-diagnostics.contracts.json" > "$report/artifacts/slice/after-046accf6.contracts.json.gz"
cp "$out/slice-before/console.log" "$report/artifacts/slice/before-046accf6.console.log"
cp "$out/slice-after/console.log" "$report/artifacts/slice/after-046accf6.console.log"
python3 "$report/scripts/slice_tally.py" "$out/slice-after/resource-diagnostics.json" \
  --before "$out/slice-before/resource-diagnostics.json" > "$report/artifacts/slice/after-vs-before.json"
cp "$rw/slice-before-vs-published.json" "$report/artifacts/slice/before-vs-published-backlog-custody.json"
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); json.dump(d['refuted_contracts'], sys.stdout, indent=1); print()" \
  "$out/slice-after/resource-diagnostics.contracts.json" > "$report/artifacts/slice/refuted-contracts-section.json"
if cmp -s "$out/slice-after/resource-diagnostics.json" "$out/slice-after-jobs1/resource-diagnostics.json" \
  && cmp -s "$out/slice-after/resource-diagnostics.contracts.json" "$out/slice-after-jobs1/resource-diagnostics.contracts.json"; then
  echo '{"jobs_8_vs_jobs_1": "resource-diagnostics.json and its contracts sidecar byte-identical"}' > "$report/artifacts/slice/jobs-independence.json"
else
  echo '{"jobs_8_vs_jobs_1": "DIFFER"}' > "$report/artifacts/slice/jobs-independence.json"
fi
echo archived
