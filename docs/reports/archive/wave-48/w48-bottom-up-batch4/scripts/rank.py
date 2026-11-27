#!/usr/bin/env python3
"""Run the unchanged original planner on both fresh corpus reports."""
from pathlib import Path
import gzip
import json
import subprocess
import sys

repo = Path(__file__).resolve().parents[4]
report = Path(__file__).resolve().parents[1]
private = repo/'private/w48-bottom-up-batch4'
records = {}
for side in ['before', 'after']:
    output = private/(side+'-plan.json')
    command = [sys.executable, str(repo/'docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py'), str(private/('corpus-'+side+'-final')/'corpus/resource-diagnostics.json'), str(output), '--top', '20']
    with (private/(side+'-plan.log')).open('w') as stream:
        run = subprocess.run(command, stdout=stream, stderr=stream)
    records[side] = dict(command=command, exit=run.returncode, log=str(private/(side+'-plan.log')))
    if run.returncode:
        raise RuntimeError(records[side])
    data = output.read_bytes()
    (report/'artifacts'/(side+'-plan.json.gz')).write_bytes(gzip.compress(data,mtime=0))
    plan = json.loads(data)
    records[side]['plan_keys'] = list(plan)
(report/'ranking-receipt.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps(records,indent=2))
