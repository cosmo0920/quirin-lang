"""Unit/function status tallies and diagnostic counts of each archived wave-45 arbiter baseline."""
import collections, glob, gzip, json
for path in sorted(glob.glob('docs/reports/w4[45]-*/integration/artifacts/diagnostics.json.gz')):
    d = json.load(gzip.open(path))
    u = collections.Counter(x['status'] for x in d['units'])
    f = collections.Counter(y['status'] for x in d['units'] for y in x['functions'])
    print(path.split('/')[2], dict(sorted(u.items())), dict(sorted(f.items())), d['diagnostic_counts'])
