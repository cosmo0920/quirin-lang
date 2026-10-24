#!/usr/bin/env python3
"""Retain complete native artifacts and raw deltas without erasure or attribution."""
import argparse
import collections
import gzip
import hashlib
import json
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
difference_source = ROOT / 'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'
differences = runpy.run_path(str(difference_source))['differences']
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('base', type=Path)
p.add_argument('tip', type=Path)
p.add_argument('out', type=Path)
for side in ['base', 'tip']:
    p.add_argument('--' + side + '-null', type=Path, required=True)
    p.add_argument('--' + side + '-outer', type=Path, required=True)
a = p.parse_args()
assert '/private/' in str(a.out.resolve()), 'complete payloads stay private'
a.out.mkdir(parents=True, exist_ok=False)
def retain(path, value):
    path.write_bytes(gzip.compress((json.dumps(value, sort_keys=True) + '\n').encode(), mtime=0))
documents = {}
manifest = {'producer': {
    'collector_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    'difference_helper_sha256': hashlib.sha256(difference_source.read_bytes()).hexdigest(),
    'operation': 'raw recursive differences only; no erase or normalize call'}}
for side in ['base', 'tip']:
    directory = getattr(a, side)
    null = getattr(a, side + '_null')
    paths = {name: directory / name for name in [
        'resource-diagnostics.json', 'resource-diagnostics.contracts.json',
        'resource-diagnostics.chunk-contracts.json', 'resource-diagnostics.source-effects.json',
        'monkey-source.json', 'run.json', 'completion.json', 'invocation.json']}
    paths.update(null=null, null_summary=null.with_suffix('.summary.json'), outer=getattr(a, side + '_outer'))
    destination = a.out / side
    destination.mkdir()
    values = {}
    manifest[side] = {}
    for name, path in paths.items():
        if not path.is_file():
            assert name not in {'resource-diagnostics.json', 'resource-diagnostics.contracts.json', 'null', 'null_summary', 'outer'}, str(path)
            continue
        raw = path.read_bytes()
        (destination / (name + '.gz')).write_bytes(gzip.compress(raw, mtime=0))
        manifest[side][name] = dict(source=str(path.resolve()), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
        values[name] = [json.loads(line) for line in raw.splitlines() if line] if name == 'null' else json.loads(raw)
    documents[side] = values
raw = differences(documents['base'], documents['tip'])
retain(a.out / 'complete-raw-differences.json.gz', raw)
left, right = [documents[side]['null'] for side in ['base', 'tip']]
keys = lambda rows: [(r['library'], r['file'], r['function']) for r in rows]
assert keys(left) == keys(right), 'null denominator or unit order changed'
changed = [dict(index=i, key=key, before=b, after=t) for i, (key, b, t) in enumerate(zip(keys(left), left, right)) if b != t]
retain(a.out / 'complete-changed-null-rows.json.gz', changed)
summary = dict(raw_differences=len(raw), null_rows=len(left), changed_null_rows=len(changed),
    null_before=dict(collections.Counter(r['null']['verdict'] for r in left)),
    null_after=dict(collections.Counter(r['null']['verdict'] for r in right)),
    null_status_changes=[dict(index=x['index'], key=x['key'], before=x['before']['null']['verdict'], after=x['after']['null']['verdict']) for x in changed if x['before']['null']['verdict'] != x['after']['null']['verdict']],
    all_raw_fields_retained=True, attribution='Pending review of complete raw differences; no cause assigned or identity erased.')
(a.out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
(a.out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary))
