"""Read a `current` report (read-only) into a pickle of (unit, function) rows.

usage: extract.py REPORT.json OUT.pkl
"""
import json, pickle, sys
R = sys.argv[1]
d = json.load(open(R))
print(d.keys() if isinstance(d, dict) else type(d))
units = d['units']
print(len(units), list(units[0].keys()))
rows = []
for u in units:
    uid = {k: u[k] for k in u if k != 'functions'}
    for f in u.get('functions', []):
        rows.append((uid, f))
print(len(rows))
pickle.dump(rows, open(sys.argv[2], 'wb'))
