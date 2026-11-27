"""Per unit holding a predicted function: status counts of its other rows.

Usage: units.py REPORT NAME...
"""
import collections as C, json, sys

d = json.load(open(sys.argv[1]))
names = set(sys.argv[2:])
for u in d['units']:
    fs = [f['function'] for f in u['functions']]
    if names & set(fs):
        others = C.Counter(f['status'] for f in u['functions'] if f['function'] not in names)
        print(u['file'], u['status'], sorted(names & set(fs)), dict(others))
