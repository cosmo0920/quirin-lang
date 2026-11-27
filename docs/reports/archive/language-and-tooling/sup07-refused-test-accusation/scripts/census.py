#!/usr/bin/env python3
"""census.py REPORT [LABEL] -- MONKEY-VHOST-PROPERTY-CUSTODY's census of one
resource-diagnostics report (corpus arbiter, system route or tail-pause slice).

Prints, as JSON: function status totals (with the not-projected count when the
report has one), the report's sha256, and the leaves this card can move:

* `mk_string_dup`: every `call to uncontracted function `mk_string_dup`` leaf;
* `owns_linked`: every refusal of a publication of an object that owns linked
  objects (gateway rule 10d's withdrawal, the shape the carry rule decides);
* `container_other`: every publication refused for its container;
* the leak and violation leaves (`leak`, `release_after_escape`, ...),
  per function, so an AFTER can classify every added one.
"""
import collections
import hashlib
import json
import sys

path = sys.argv[1]
raw = open(path, 'rb').read()
report = json.loads(raw)
status = collections.Counter()
leaves = collections.defaultdict(list)
violations = collections.defaultdict(list)
for unit in report['units']:
    for function in unit.get('functions', []):
        status[function['status']] += 1
        name = f"{unit['file']}::{function['function']}"
        for diagnostic in function.get('diagnostics', []):
            text = diagnostic['description']
            where = diagnostic.get('origin') or diagnostic.get('line')
            if 'uncontracted function `mk_string_dup`' in text:
                leaves['mk_string_dup'].append((name, where))
            if 'which owns linked objects' in text:
                leaves['owns_linked'].append((name, where))
            if 'into a container this function does not show outlives it' in text:
                leaves['container_other'].append((name, where))
            if diagnostic['category'] not in ('unsupported', 'unknown_alias'):
                violations[name].append((diagnostic['category'], where, text[:120]))
print(json.dumps({
    'report': path,
    'sha256': hashlib.sha256(raw).hexdigest(),
    'status': dict(sorted(status.items())),
    'diagnostic_counts': report.get('diagnostic_counts'),
    'leaves': {key: {'count': len(value), 'functions': sorted({n for n, _ in value}), 'sites': value}
               for key, value in sorted(leaves.items())},
    'violation_functions': len(violations),
    'violations': violations,
}, indent=1, sort_keys=True))
