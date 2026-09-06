#!/usr/bin/env python3
"""Check exact audit parity, allowing only the three recorded artifact digests."""
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
BASE = HERE.parent / 'w4-contextual-recursion'


def differences(left, right, path=''):
    if type(left) is not type(right):
        return [(path, left, right)]
    if isinstance(left, dict):
        assert left.keys() == right.keys(), path
        return sum((differences(left[k], right[k], path + '/' + k) for k in left), [])
    if isinstance(left, list):
        assert len(left) == len(right), path
        return sum((differences(x, y, path + '/' + str(i)) for i, (x, y) in enumerate(zip(left, right))), [])
    return [] if left == right else [(path, left, right)]


def main():
    expected = json.loads((HERE / 'audit-parity.json').read_text())
    packages = json.loads((HERE / 'packages-a.json').read_text())
    digest = next(item['executable_sha256'] for item in packages['artifacts'] if item['plugin_id'] == 'qcore.chunkio')
    for name, check in expected['checks'].items():
        old = gzip.decompress((BASE / (name + '.json.gz')).read_bytes())
        new = gzip.decompress((HERE / (name + '.json.gz')).read_bytes())
        assert hashlib.sha256(old).hexdigest() == check['baseline_sha256']
        assert hashlib.sha256(new).hexdigest() == check['new_sha256']
        if name != 'source-effects':
            assert old == new, name
            continue
        before, after = json.loads(old), json.loads(new)
        changed = differences(before, after)
        allowed = [f'/results/{i}/evidence/reproducibility/package_digest'
                   for i, row in enumerate(before['results']) if row['status'] == 'checked_function_contract']
        assert len(allowed) == 3
        assert {entry[0] for entry in changed} == set(allowed)
        assert len({entry[1] for entry in changed}) == 1
        assert all(entry[2] == digest for entry in changed)
        assert [{'json_pointer': p, 'old': a, 'new': b} for p, a, b in changed] == check['exact_changed_leaves']
        assert old.replace(changed[0][1].encode(), digest.encode()) == new
        assert sum(row['status'] == 'unknown' for row in after['results']) == 93
    print('Three audit outputs byte-identical; source-effects differs only in three verified package digests.')


if __name__ == '__main__':
    main()
