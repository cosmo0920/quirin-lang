#!/usr/bin/env python3
"""Archive final route comparisons without rebuilding or rerunning an audit."""
from pathlib import Path
import gzip
import json
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]
BASE = Path('/tmp/qcore-fb-lifetime-s3/private/fb-lifetime-s3/measure-final-1')
TIP = REPO / 'private/detach-same-item/measure-final-2'
OUT = REPO / 'private/detach-same-item/final-review'
OUT.mkdir(exist_ok=True)
commands = json.loads((OUT / 'commands.json').read_text()) if (OUT / 'commands.json').exists() else []
routes = sys.argv[1:] or ['corpus', 'system', 'slice']
assert routes and len(set(routes)) == len(routes) and set(routes) <= {'corpus','system','slice'}


def run(label, argv):
    result = subprocess.run(argv, cwd=REPO, capture_output=True)
    (OUT / (label + '.log')).write_bytes(result.stdout + result.stderr)
    (OUT / (label + '.exit')).write_text(str(result.returncode) + '\n')
    commands.append(dict(name=label, command=argv, exit=result.returncode,
                         private_log=str(OUT / (label + '.log'))))
    (OUT / 'commands.json').write_text(json.dumps(commands, indent=2) + '\n')
    if result.returncode:
        raise SystemExit(result.returncode)


for route in routes:
    a, b = [root / route / ('corpus/resource-diagnostics.json' if route == 'corpus'
                            else 'resource-diagnostics.json') for root in [BASE, TIP]]
    assert int((TIP / (route + '.exit')).read_text()) == 0
    run(route + '-archive', [sys.executable, str(REPORT / 'scripts/archive.py'), str(a), str(b), route])
    run(route + '-fields', [sys.executable, str(REPORT / 'scripts/review_fields.py'), route])
    run(route + '-precise', [sys.executable, str(REPORT / 'scripts/review_precise.py'), route])
    run(route + '-projections', [sys.executable, str(REPORT / 'scripts/projections.py'),
                                str(BASE / route), str(TIP / route), route])

if 'corpus' not in routes:
    raise SystemExit(0)

names = {'cJSON_DetachItemViaPointer', 'cJSON_DetachItemFromArray',
         'cJSON_DetachItemFromObject', 'cJSON_DetachItemFromObjectCaseSensitive',
         'cJSON_DeleteItemFromArray', 'cJSON_DeleteItemFromObject',
         'cJSON_DeleteItemFromObjectCaseSensitive'}
selected = {}
for side, root in [('before', BASE), ('after', TIP)]:
    data = json.loads((root / 'corpus/corpus/resource-diagnostics.json').read_text())
    found = []
    for unit in data['units']:
        for row in unit['functions']:
            if row['function'] in names:
                found.append(dict(library=unit['library'], file=unit['file'], **row))
    assert len(found) == len(names) and {r['function'] for r in found} == names
    selected[side] = found
(REPORT / 'artifacts/seven-body-review.json.gz').write_bytes(
    gzip.compress((json.dumps(selected, indent=2) + '\n').encode(), mtime=0))
baseline_plan = REPO / 'private/detach-same-item/plan-s3-baseline.json'
(REPORT / 'artifacts/before-plan.json.gz').write_bytes(gzip.compress(baseline_plan.read_bytes(), mtime=0))
for side, root in [('after', TIP)]:
    plan = OUT / (side + '-plan.json')
    run(side + '-plan', [sys.executable,
        str(REPO / 'docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py'),
        str(root / 'corpus/corpus/resource-diagnostics.json'), str(plan), '--top', '20'])
    (REPORT / ('artifacts/' + side + '-plan.json.gz')).write_bytes(
        gzip.compress(plan.read_bytes(), mtime=0))
