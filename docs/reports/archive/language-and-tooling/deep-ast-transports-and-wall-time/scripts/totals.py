"""totals.py REPORT-OR-DIR -- RDHTTP-HEADER-SUMMARY-GAP: a route report's row
statuses (Safe / Unsafe / Unknown / Pending ...) and its sha256."""
import collections
import hashlib
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
if path.is_dir():
    path = path / 'resource-diagnostics.json'
data = path.read_bytes()
report = json.loads(data)
tally = collections.Counter()
for unit in report['units']:
    for row in unit.get('functions') or []:
        tally[row.get('status')] += 1
print(hashlib.sha256(data).hexdigest(), dict(sorted(tally.items())))
