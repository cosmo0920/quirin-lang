"""Filter a compile database to the listed source files (read-only input).

usage: filter_db.py COMPILE_COMMANDS.json CHECKOUT_ROOT UNITS.txt OUT.json

UNITS.txt lists checkout-relative paths, one per line. Every entry whose
`file` is CHECKOUT_ROOT/<path> for a listed path is kept, in order.
"""
import json, sys

db = json.load(open(sys.argv[1]))
root = sys.argv[2].rstrip('/') + '/'
units = {line.strip() for line in open(sys.argv[3]) if line.strip()}
kept = [entry for entry in db if entry['file'].startswith(root) and entry['file'][len(root):] in units]
found = {entry['file'][len(root):] for entry in kept}
missing = sorted(units - found)
json.dump(kept, open(sys.argv[4], 'w'), indent=1)
print(f'{len(kept)} entries for {len(found)} of {len(units)} units; missing: {missing}')
