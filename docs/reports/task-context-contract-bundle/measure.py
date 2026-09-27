#!/usr/bin/env python3
"""Reproduce source-byte estimates and preservation checks; no token telemetry."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('task_context', ROOT/'scripts/task_context.py')
context = importlib.util.module_from_spec(spec)
spec.loader.exec_module(context)
baseline = json.loads((OUT/'baseline.json').read_text())
base = baseline['base']

if any((ROOT / "providers").glob("*/contracts/**/*.qv")):
    raise SystemExit("Historical checkpoint only: this replay pins the pre-migration 96-module corpus. "
                     "Use the recorded report revision; it cannot measure the current .qv corpus.")

def old(path):
    return subprocess.check_output(['git', '-C', str(ROOT), 'show', f'{base}:{path}'])

operations = (OUT/'operations-before.txt').read_bytes()
assert operations == old('docs/agents/operations.md')
assert hashlib.sha256(operations).hexdigest() == baseline['operations_sha256']
common = ['AGENTS.md', 'CLAUDE.md', 'docs/agents/operations.md', 'docs/STATUS.md',
          'docs/agents/README.md', 'docs/agents/validation.md', 'docs/agents/evidence.md']
samples = [
    ('documentation', {'docs/README.md'}, ['docs'], []),
    ('provider_contract', {'providers/qcore-provider-cmetrics/contracts/cmetrics/counter_reset.qcore'}, ['contracts', 'providers'], ['gateway', 'source-identity', 'diagnostics', 'providers']),
    ('gateway', {'crates/qcore-c-gateway/src/typed_source.rs'}, ['gateway'], ['gateway', 'source-identity', 'diagnostics']),
    ('cross_component', {'crates/qcore-c-gateway/src/typed_source.rs', 'crates/qcore-verifier-host/src/lib.rs'}, ['gateway', 'providers', 'concurrency'], ['gateway', 'source-identity', 'diagnostics', 'providers', 'concurrency', 'buffers-records', 'source-routes']),
]
rows=[]
for name, paths, topics, pages in samples:
    before_paths = sorted(set(common + [f'docs/agents/{page}.md' for page in pages]))
    before = sum(len(old(path)) for path in before_paths)
    packet = context.packet(ROOT, paths, topics)
    after = packet['metrics']['unique_selected_bytes'] + (ROOT/'CLAUDE.md').stat().st_size
    rows.append({'task':name,'paths':sorted(paths),'topics':topics,'before_paths':before_paths,
                 'after_readings':packet['readings'],'before_unique_bytes':before,'after_unique_bytes':after,
                 'before_single_pass_retrieval_bytes':before,'after_single_pass_retrieval_bytes':after,
                 'reduction_percent':round(100*(before-after)/before,2)})
contracts=[]
for path in sorted((ROOT/'providers').glob('*/contracts/**/*.qcore')):
    relative=path.relative_to(ROOT).as_posix()
    assert path.read_bytes() == old(relative), relative
    contracts.append({'path':relative,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
# Includes provider-owned and shared contract trees, without checking new files
# against a hand-maintained duplicate snapshot.
assert len(contracts)==96
result={'base':base,'assumed_injected_bytes':{'before':sum(len(old(p)) for p in ['AGENTS.md','CLAUDE.md']),
        'after':sum((ROOT/p).stat().st_size for p in ['AGENTS.md','CLAUDE.md'])},
        'operations_bytes':{'before':len(operations),'after':(ROOT/'docs/agents/operations.md').stat().st_size},
        'samples':rows,'contract_byte_parity':contracts,
        'boundary':'Synthetic once-per-section packets, not actual session retrieval, billed tokens, runtime latency, or safety coverage. Broad/unknown scope can select all topics. No 185-unit replay is claimed by this measurement.'}
(OUT/'measurements.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({**{k:v for k,v in result.items() if k not in ['samples','contract_byte_parity']},'samples':[{k:r[k] for k in ['task','before_unique_bytes','after_unique_bytes','reduction_percent']} for r in rows],'contracts_unchanged':len(contracts)},indent=2))
