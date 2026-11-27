#!/usr/bin/env python3
"""Archive completed staged audits and itemize every diagnostic/verdict change.

Run from the repository root after all five audits have completed:
    python3 docs/reports/c-gateway-wave/evidence.py
The canonical five-artifact itemizer is invoked for each adjacent stage.
"""
import collections
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
REPORT = Path(__file__).resolve().parent
RUNS = ROOT / 'measure/c-gateway-wave'
STAGES = {'before': 'before', 'scalar': 'scalar', 'frame': 'frame', 'joined': 'final', 'after': 'hardened'}
ARTIFACTS = {
    'diagnostics': 'resource-diagnostics.json',
    'source': 'monkey-source.json',
    'chunk-contracts': 'resource-diagnostics.chunk-contracts.json',
    'source-effects': 'resource-diagnostics.source-effects.json',
    'contracts': 'resource-diagnostics.contracts.json',
}


def dump(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def inventory(document):
    units, functions, leaves = {}, {}, set()
    for unit in document['units']:
        key = (unit['library'], unit['file'])
        units[key] = unit['status']
        for function in unit['functions']:
            fkey = key + (function['function'],)
            functions[fkey] = function['status']
            for diagnostic in function['diagnostics']:
                leaves.add(fkey + (diagnostic['line'], diagnostic['category'], diagnostic['description']))
    return units, functions, leaves


def differences(before, after):
    bu, bf, bl = inventory(before)
    au, af, al = inventory(after)
    def statuses(b, a):
        return [{'key': k, 'before': b.get(k), 'after': a.get(k)}
                for k in sorted(b.keys() | a.keys()) if b.get(k) != a.get(k)]
    return {
        'counts_before': before['diagnostic_counts'],
        'counts_after': after['diagnostic_counts'],
        'units': statuses(bu, au), 'functions': statuses(bf, af),
        'removed': sorted(bl - al), 'added': sorted(al - bl),
        'meaning': 'Diagnostic occurrences; Unsafe remains provisional. Coordinates are internal preprocessed lines.',
    }


def main():
    assert json.loads((RUNS / 'hardened/run.json').read_text())['exit'] == 0
    receipts, documents, headlines = {}, {}, {}
    for stage, run in STAGES.items():
        output = REPORT / ('artifacts' if stage == 'after' else f'artifacts-{stage}')
        output.mkdir(exist_ok=True)
        receipts[stage] = {}
        for stem, filename in ARTIFACTS.items():
            raw = (RUNS / run / filename).read_bytes()
            packed = gzip.compress(raw, compresslevel=9, mtime=0)
            (output / f'{stem}.json.gz').write_bytes(packed)
            receipts[stage][stem] = {'sha256': hashlib.sha256(raw).hexdigest(),
                                    'gzip_sha256': hashlib.sha256(packed).hexdigest(), 'bytes': len(raw)}
        documents[stage] = json.loads((RUNS / run / ARTIFACTS['diagnostics']).read_text())
        units, functions, _ = inventory(documents[stage])
        headlines[stage] = {'units': len(units), 'functions': len(functions),
                            'unit_statuses': dict(collections.Counter(units.values())),
                            'function_statuses': dict(collections.Counter(functions.values())),
                            'diagnostics': documents[stage]['diagnostic_counts']}
        binary = RUNS / f'audit-{run}'
        receipts[stage]['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    baseline = ROOT / 'docs/reports/w4-recursive-summaries/artifacts'
    for stem in ['diagnostics', 'chunk-contracts']:
        assert gzip.decompress((baseline / f'{stem}.json.gz').read_bytes()) == (RUNS / 'before' / ARTIFACTS[stem]).read_bytes(), stem
    for stem in ['source', 'source-effects', 'contracts', 'chunk-contracts']:
        assert len({record[stem]['sha256'] for record in receipts.values()}) == 1, stem
    before_units, before_functions, _ = inventory(documents['before'])
    after_units, after_functions, _ = inventory(documents['after'])
    for before, after in [(before_units, after_units), (before_functions, after_functions)]:
        assert before.keys() == after.keys(), 'scope changed'
        assert not any(before[key] != 'Safe' and value == 'Safe' for key, value in after.items()), 'Safe promotion'
    for before, after in [('before', 'scalar'), ('scalar', 'frame'), ('frame', 'joined'), ('joined', 'after'), ('before', 'after')]:
        change = differences(documents[before], documents[after])
        dump(REPORT / f'leaves-{before}-to-{after}.json', change)
        subprocess.run(['python3', str(ROOT / 'docs/reports/corpus-repin/diff_artifacts.py'),
                        '--archived', str(REPORT / f'artifacts-{before}'),
                        '--measured', str(RUNS / STAGES[after]),
                        '--output', str(REPORT / f'diff-{before}-to-{after}.json')], check=True,
                       stdout=subprocess.DEVNULL)
    subprocess.run(['python3', str(ROOT / 'docs/reports/corpus-repin/diff_artifacts.py'),
                    '--archived', str(baseline), '--measured', str(RUNS / 'before'),
                    '--output', str(REPORT / 'diff-baseline-to-before.json')], check=True,
                   stdout=subprocess.DEVNULL)
    for path in REPORT.glob('diff-*.json'):
        path.with_suffix('.json.gz').write_bytes(gzip.compress(path.read_bytes(), compresslevel=9, mtime=0))
        path.unlink()
    dump(REPORT / 'artifact-manifest.json', receipts)
    dump(REPORT / 'headlines.json', headlines)
    print(json.dumps(headlines, indent=2))


if __name__ == '__main__':
    main()
