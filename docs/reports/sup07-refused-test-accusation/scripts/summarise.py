#!/usr/bin/env python3
"""summarise.py OUT_DIR LABEL... -- run census.py over each label's corpus,
system and slice reports (private/sup07/LABEL, private/sup07/LABEL-system and the
shared slice attempt's sup07-LABEL), write OUT_DIR/LABEL-{corpus,system,slice}.json
and print one summary line per report."""
import json
import pathlib
import subprocess
import sys

here = pathlib.Path(__file__).resolve().parent
repo = here.parents[3]
slices = pathlib.Path('/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1')
out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
for label in sys.argv[2:]:
    reports = {
        'corpus': repo / 'private/sup07' / label / 'resource-diagnostics.json',
        'system': repo / 'private/sup07' / f'{label}-system' / 'resource-diagnostics.json',
        'slice': slices / f'sup07-{label}' / 'resource-diagnostics.json',
    }
    for route, path in reports.items():
        text = subprocess.run([sys.executable, str(here / 'census.py'), str(path)],
                              check=True, capture_output=True, text=True).stdout
        (out / f'{label}-{route}.json').write_text(text)
        data = json.loads(text)
        leaves = {key: (value['count'], len(value['functions'])) for key, value in data['leaves'].items()}
        print(label, route, data['sha256'][:8], data['status'], data['diagnostic_counts'], leaves,
              'violation functions', data['violation_functions'])
