#!/usr/bin/env python3
"""Check exact coverage of manually reviewed differences and their source bytes.

This verifies record identity, not the correctness of the written source review.
Usage: check_attribution.py CORPUS_SOURCE SYSTEM_SOURCE
"""
import collections
import hashlib
import json
import pathlib
import sys

report = pathlib.Path(__file__).resolve().parents[1]
reviews = json.loads((report / 'census/reviewed-attribution.json').read_text())
roots = dict(zip(['corpus', 'system'], map(pathlib.Path, sys.argv[1:])))
assert len(roots) == 2, 'both source roots are required'
key = lambda item: json.dumps(item, sort_keys=True)
result = {}
for scope, root in roots.items():
    auto = json.loads((report / f'census/{scope}-attribution.json').read_text())
    selected = [r for r in reviews if r['scope'] == scope]
    assert collections.Counter(map(key, auto['unattributed'])) == collections.Counter(key(r['observation']) for r in selected), 'unreviewed or stale difference'
    for review in selected:
        assert review['reason'] and review['callees'] and review['source_evidence']
        for evidence in review['source_evidence']:
            raw = (root / evidence['file']).read_bytes()
            assert hashlib.sha256(raw).hexdigest() == evidence['sha256']
            excerpt = '\n'.join(raw.decode().splitlines()[evidence['first_line']-1:evidence['last_line']])
            assert excerpt == evidence['text']
    assert not auto['corrections'], 'a Safe correction requires separate review'
    assert not auto['new_leak_or_violation_leaves'], 'new violations require separate review'
    result[scope] = dict(automatic_unattributed=len(auto['unattributed']), reviewed=len(selected), remaining_unattributed=0, source_identity_checked=True)
print(json.dumps(result, indent=2))
