"""Run the census driver once per preprocessed unit.

Usage: run_census.py DRIVER PP_DIR OUT_DIR [JOBS [UNIT_FILTER]]
Each unit gets its own process and its own stream file OUT_DIR/<unit>.jsonl
(and OUT_DIR/<unit>.verdicts.json). The leaf rows (`dump.py`) are read from
$SCALAR_FORM_ROWS (default rows.json); each unit's entry is one of its
functions with a leaf. Rule switches pass through the environment
(QCORE_GATEWAY_RULES_OFF=41 for the without-rule side).
"""
import json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

driver, pp, out = sys.argv[1], sys.argv[2], sys.argv[3]
jobs = int(sys.argv[4]) if len(sys.argv) > 4 else 6
os.makedirs(out, exist_ok=True)
flt = sys.argv[5] if len(sys.argv) > 5 else ''
rows = json.load(open(os.environ.get('SCALAR_FORM_ROWS', 'rows.json')))
entry = {r['unit'].replace('/', '__') + '.i': r['fn'] for r in rows}
units = sorted(f for f in os.listdir(pp) if f.endswith('.i') and flt in f)


def one(name):
    stream = os.path.join(out, name[:-2] + '.jsonl')
    if os.path.exists(stream):
        os.remove(stream)
    env = dict(os.environ, QCORE_SCALAR_FORM_UNIT=os.path.join(pp, name), QCORE_SCALAR_FORM_CENSUS=stream,
               QCORE_SCALAR_FORM_VERDICTS=os.path.join(out, name[:-2] + '.verdicts.json'),
               QCORE_SCALAR_FORM_ENTRY=entry[name])
    p = subprocess.run([driver, '--ignored', '--exact', 'census_one_unit', '--nocapture'], env=env,
                       capture_output=True, text=True, timeout=7200)
    with open(os.path.join(out, name[:-2] + '.log'), 'w') as log:
        log.write(p.stdout + p.stderr)
    return name, p.returncode


with ThreadPoolExecutor(max_workers=jobs) as ex:
    for name, code in ex.map(one, units):
        print(code, name, flush=True)
