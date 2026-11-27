"""Do two `prepare --link` outputs decide every symbol alike?

usage: same_decisions.py LINK_RESOLUTION_DIR_A LINK_RESOLUTION_DIR_B

Compares the candidate maps' symbol lists and the refusal lists (function,
reason), ignoring the premise and digests, which name each run's own inputs.
"""
import json
import sys
from pathlib import Path


def load(root):
    root = Path(root)
    candidate = json.loads((root / 'link-map.candidate.json').read_text())
    refusals = json.loads((root / 'link-map.candidate.refusals.json').read_text())
    return (candidate['symbols'],
            [(r['function'], r['reason']) for r in refusals['refused']])


a, b = load(sys.argv[1]), load(sys.argv[2])
print(json.dumps({'same_derived': a[0] == b[0], 'same_refusals': a[1] == b[1],
                  'derived': len(a[0]), 'refused': len(a[1])}))
