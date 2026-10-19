#!/usr/bin/env python3
"""Archive full baseline/tip payloads and compare each property separately."""
from pathlib import Path
import argparse
import gzip
import json
import subprocess
import sys

REPORT = Path(__file__).resolve().parents[1]
REPO = REPORT.parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseline', type=Path)
parser.add_argument('tip', type=Path)
parser.add_argument('routes', nargs='*', choices=['corpus', 'system', 'slice'])
args = parser.parse_args()
routes = args.routes or ['corpus', 'system', 'slice']
assert len(routes) == len(set(routes))
out = REPO / 'private/detach-lookup-membership/final-review'
out.mkdir(exist_ok=True)
receipt = out / 'commands.json'
commands = json.loads(receipt.read_text()) if receipt.exists() else []


def run(label, argv):
    result = subprocess.run(argv, cwd=REPO, capture_output=True)
    (out / (label + '.log')).write_bytes(result.stdout + result.stderr)
    (out / (label + '.exit')).write_text(str(result.returncode) + '\n')
    commands.append(dict(name=label, command=argv, exit=result.returncode,
                         private_log=str(out / (label + '.log'))))
    receipt.write_text(json.dumps(commands, indent=2) + '\n')
    if result.returncode:
        raise SystemExit(result.returncode)


for route in routes:
    paths = [root / route / ('corpus/resource-diagnostics.json' if route == 'corpus'
                             else 'resource-diagnostics.json')
             for root in [args.baseline, args.tip]]
    assert int((args.tip / (route + '.exit')).read_text()) == 0
    run(route + '-archive', [sys.executable, str(REPORT / 'scripts/archive.py'),
                            *map(str, paths), route])
    for tool in ['review_fields', 'review_precise']:
        run(route + '-' + tool, [sys.executable, str(REPORT / ('scripts/' + tool + '.py')), route])
    run(route + '-projections', [sys.executable, str(REPORT / 'scripts/projections.py'),
                                str(args.baseline / route), str(args.tip / route), route])
    if route != 'corpus':
        continue
    names = {'get_array_item', 'cJSON_GetArrayItem', 'cJSON_DetachItemViaPointer',
             'cJSON_DetachItemFromArray', 'cJSON_DetachItemFromObject',
             'cJSON_DetachItemFromObjectCaseSensitive', 'cJSON_DeleteItemFromArray',
             'cJSON_DeleteItemFromObject', 'cJSON_DeleteItemFromObjectCaseSensitive'}
    selected = {}
    for side, path in zip(['before', 'after'], paths):
        data = json.loads(path.read_bytes())
        selected[side] = [dict(library=unit['library'], file=unit['file'], **row)
                          for unit in data['units'] for row in unit['functions']
                          if row['function'] in names]
        assert {r['function'] for r in selected[side]} == names
        plan = out / (side + '-plan.json')
        run(side + '-plan', [sys.executable,
            str(REPO / 'docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py'),
            str(path), str(plan), '--top', '20'])
        (REPORT / ('artifacts/' + side + '-plan.json.gz')).write_bytes(
            gzip.compress(plan.read_bytes(), mtime=0))
    (REPORT / 'artifacts/nine-body-review.json.gz').write_bytes(
        gzip.compress((json.dumps(selected, indent=2) + '\n').encode(), mtime=0))
