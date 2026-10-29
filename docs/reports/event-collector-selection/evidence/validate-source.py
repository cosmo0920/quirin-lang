#!/usr/bin/env python3
"""Replay archived collector source reports using the built source-audit example.

This checks source-import/report reproducibility under the recorded native Clang
scope. It supplies no lifetime verdict, build equivalence or binary attestation.
"""
import argparse
import gzip
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--receipt', type=Path, default=Path(__file__).with_name('source-receipt.json'))
args = parser.parse_args()
receipt = json.loads(args.receipt.read_text())
assert receipt['schema'] == 'qcore.collector-source-archive/1'
base = args.receipt.resolve().parent
contents = {}
for entry in receipt['files']:
    relative = Path(entry['file'])
    assert not relative.is_absolute() and '..' not in relative.parts
    assert entry['file'] not in contents
    data = (base / relative).read_bytes()
    assert len(data) == entry['bytes'] and hashlib.sha256(data).hexdigest() == entry['sha256']
    raw = gzip.decompress(data)
    assert len(raw) == entry['raw_bytes'] and hashlib.sha256(raw).hexdigest() == entry['raw_sha256']
    contents[entry['file']] = raw
binary = args.binary.resolve()
reports = []
with tempfile.TemporaryDirectory(prefix='qcore-collector-archive-') as directory:
    temporary = Path(directory)
    query = temporary / 'query.json'
    query.write_bytes(contents['run/production-query.json.gz'])
    for stem in ['production-0', 'production-1', 'production-corpus-0', 'production-corpus-1', 'production-slice-0', 'production-slice-1']:
        source = temporary / (stem + '.i')
        source.write_bytes(contents['source/' + stem + '.i.gz'])
        result = subprocess.run([str(binary), str(source), str(query)], capture_output=True, check=True)
        actual = json.loads(result.stdout)
        expected = json.loads(contents['final/' + stem + '.json.gz'])
        assert actual == expected, f'{stem}: complete source report differs'
        assert 'Ok' in actual['selector'] and 'Ok' in actual['conditional_loop']
        reports.append(stem)
print(json.dumps({'source_checkpoint': receipt['source_checkpoint'], 'verified_archives': len(contents), 'replayed_configurations': len(reports), 'imports': len(reports) * 2, 'internal_source_replays': len(reports) * 2, 'remaining_report_deltas': 0, 'lifetime_verdict': 'not established'}, indent=2))
