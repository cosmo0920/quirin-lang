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
  "$rw/arbiter-368/resource-diagnostics.contracts.json" > "$report/artifacts/arbiter/contracts-delta.json"
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); json.dump(d['refuted_contracts'], sys.stdout, indent=1); print()" \
  "$rw/arbiter-368/resource-diagnostics.contracts.json" > "$report/artifacts/arbiter/refuted-contracts-section.json"
zcat docs/reports/w45-container-root-reach/integration/artifacts/contracts.json.gz > "$rw/baseline-contracts.json"
python3 "$report/scripts/sidecar_delta.py" "$rw/baseline-contracts.json" "$rw/arbiter-368/resource-diagnostics.contracts.json" \
  --alias "$PWD=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da" > "$report/artifacts/arbiter/contracts-delta-vs-baseline.json"
cp "$rw/arbiter-before/run.json" "$report/artifacts/arbiter/before-run.json"
cp "$rw/arbiter-368/run.json" "$report/artifacts/arbiter/after-run.json"
# Slice: BEFORE and AFTER reports, their comparison, the section, and --jobs 1.
gzip -9 -n -c "$out/slice-before/resource-diagnostics.json" > "$report/artifacts/slice/before-046accf6.json.gz"
gzip -9 -n -c "$out/slice-368/resource-diagnostics.json" > "$report/artifacts/slice/after-046accf6.json.gz"
gzip -9 -n -c "$out/slice-368/resource-diagnostics.contracts.json" > "$report/artifacts/slice/after-046accf6.contracts.json.gz"
cp "$out/slice-before/console.log" "$report/artifacts/slice/before-046accf6.console.log"
cp "$out/slice-368/console.log" "$report/artifacts/slice/after-046accf6.console.log"
python3 "$report/scripts/slice_tally.py" "$out/slice-368/resource-diagnostics.json" \
  --before "$out/slice-before/resource-diagnostics.json" > "$report/artifacts/slice/after-vs-card-base.json"
# Against `main`'s own slice (W45-CONTAINER-ROOT-REACH's archived AFTER): this
# card's movement alone.
python3 "$report/scripts/slice_tally.py" "$out/slice-368/resource-diagnostics.json" \
  --before docs/reports/w45-container-root-reach/artifacts/slice/after.json.gz > "$report/artifacts/slice/after-vs-before.json"
cp "$rw/slice-before-vs-published.json" "$report/artifacts/slice/before-vs-published-backlog-custody.json"
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); json.dump(d['refuted_contracts'], sys.stdout, indent=1); print()" \
  "$out/slice-368/resource-diagnostics.contracts.json" > "$report/artifacts/slice/refuted-contracts-section.json"
if cmp -s "$out/slice-368/resource-diagnostics.json" "$out/slice-368-jobs1/resource-diagnostics.json" \
  && cmp -s "$out/slice-368/resource-diagnostics.contracts.json" "$out/slice-368-jobs1/resource-diagnostics.contracts.json"; then
  echo '{"jobs_8_vs_jobs_1": "resource-diagnostics.json and its contracts sidecar byte-identical"}' > "$report/artifacts/slice/jobs-independence.json"
else
  echo '{"jobs_8_vs_jobs_1": "DIFFER"}' > "$report/artifacts/slice/jobs-independence.json"
fi
# A checkpointed run (W45-AUDIT-CHECKPOINTS) seals a second phase and composes
# the whole run's outputs.
if cmp -s "$out/slice-368/resource-diagnostics.json" "$out/slice-368-ckpt/resource-diagnostics.json" \
  && cmp -s "$out/slice-368/resource-diagnostics.contracts.json" "$out/slice-368-ckpt/resource-diagnostics.contracts.json"; then
  echo '{"whole_vs_checkpointed": "resource-diagnostics.json and its contracts sidecar byte-identical"}' > "$report/artifacts/slice/checkpointed.json"
else
  echo '{"whole_vs_checkpointed": "DIFFER"}' > "$report/artifacts/slice/checkpointed.json"
fi
cp "$out/slice-368-ckpt/stderr.log" "$report/artifacts/slice/checkpointed.stderr.log"
cp "$out/qo-368/checkpoint-run.json" "$report/artifacts/slice/checkpoint-run.json"
echo archived
