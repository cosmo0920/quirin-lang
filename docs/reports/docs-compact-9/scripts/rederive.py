"""Re-derive the headline figures DOCS-COMPACT-9 touches, from primary sources only."""
import collections
import glob
import gzip
import json
import subprocess
from pathlib import Path

out = {}
base = 'docs/reports/w45-container-root-reach/integration/artifacts'
d = json.load(gzip.open(f'{base}/diagnostics.json.gz'))
units = d['units'] if isinstance(d, dict) and 'units' in d else d
out['baseline'] = base
out['unit_statuses'] = dict(collections.Counter(u['status'] for u in units))
out['unit_total'] = len(units)
fs = collections.Counter(f['status'] for u in units for f in u.get('functions', []))
out['function_statuses'] = dict(fs)
out['function_total'] = sum(fs.values())
out['diagnostic_counts'] = d.get('diagnostic_counts') if isinstance(d, dict) else None
for u in units:
    for f in u.get('functions', []):
        if f.get('name') == 'flb_input_chunk_create':
            out['flb_input_chunk_create'] = f['status']

ct = json.load(open('docs/reevaluation/ct_manifest.json'))
cases = ct['cases'] if isinstance(ct, dict) else ct
out['ct_mapping'] = dict(collections.Counter(c['mapping_status'] for c in cases))
out['ct_proof'] = sorted(c['id'] for c in cases if c.get('stages', {}).get('proof', {}).get('status') == 'executed')
out['ct_sanitized'] = sorted(c['id'] for c in cases if c.get('stages', {}).get('native_sanitized', {}).get('status') == 'executed')

m = json.load(open('docs/reviews/r0_3/acceptance_matrix.json'))
nf = m['named_followups']
out['named_followups_carried'] = len(nf)
out['named_followups_dimension'] = m.get('dimensions', {}).get('named_followups')

corpus = json.load(open('contract/corpus.json'))
out['contract_corpus_keys'] = list(corpus)[:10]
mods = corpus.get('modules') or corpus.get('entries')
out['contract_corpus_modules'] = len(mods) if mods is not None else None
tracked = subprocess.check_output(['git', 'ls-files', '*.qcore']).decode().split()
out['lean_audits'] = len(glob.glob('proof/Audit*.lean'))
out['lean_toolchain'] = Path('proof/lean-toolchain').read_text().strip()
sel = json.load(open('cycle19-resource-contracts.json')) if Path('cycle19-resource-contracts.json').exists() else None
out['manifest_selected'] = len(sel['modules']) if sel else None
print(json.dumps(out, indent=1))
