#!/usr/bin/env python3
"""Archive full standard five-artifact diff and context-bounded impact for a route."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[4]
CARD = Path(__file__).resolve().parent
NAMES = {'diagnostics': 'resource-diagnostics.json', 'source': 'monkey-source.json',
         'chunk-contracts': 'resource-diagnostics.chunk-contracts.json',
         'source-effects': 'resource-diagnostics.source-effects.json',
         'contracts': 'resource-diagnostics.contracts.json'}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    archived = args.out/'standard-archived'
    archived.mkdir()
    manifest = {}
    for stem, name in NAMES.items():
        path = args.before/name
        manifest[name] = {'present': path.exists()}
        if path.exists():
            raw = path.read_bytes()
            (archived/(stem+'.json.gz')).write_bytes(gzip.compress(raw, mtime=0))
            manifest[name].update(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    (args.out/'standard-inputs.json').write_text(json.dumps(manifest, indent=2)+'\n')
    command = [sys.executable, str(ROOT/'docs/reports/corpus-repin/diff_artifacts.py'),
               '--archived', str(archived), '--measured', str(args.after),
               '--output', str(args.out/'standard-five-artifact-diff.json'), '--full']
    code = subprocess.run(command, check=False).returncode
    (args.out/'standard-command.json').write_text(json.dumps({'command': command, 'exit': code}, indent=2)+'\n')
    if code:
        return code
    command = [sys.executable, str(CARD/'impact.py'),
               str(args.after/'resource-diagnostics.contracts.json'),
               str(args.after/'resource-diagnostics.json'), str(args.out/'impact.json')]
    code = subprocess.run(command, check=False).returncode
    (args.out/'impact-command.json').write_text(json.dumps({'command': command, 'exit': code}, indent=2)+'\n')
    if code:
        return code
    impact = json.loads((args.out/'impact.json').read_text())
    (args.out/'impact-counts.json').write_text(json.dumps({
        'potential_consulting_rows': impact['potential_consulting_rows'],
        'exact_context_affected': len(impact['affected']),
        'dependent_other_contexts_unchanged': len(impact['dependent_other_contexts_unchanged']),
        'unrelated': len(impact['unrelated']), 'verdict_rewrites': 0}, indent=2)+'\n')
    for name in ['standard-five-artifact-diff.json', 'impact.json']:
        path = args.out/name
        (args.out/(name+'.gz')).write_bytes(gzip.compress(path.read_bytes(), mtime=0))
        path.unlink()
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
