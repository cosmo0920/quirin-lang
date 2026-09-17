#!/usr/bin/env python3
"""Does a measured run reproduce an archived baseline byte for byte?

Card C-GATEWAY-RETURN-POSITION's BEFORE run must reproduce the current corpus
baseline (`docs/reports/w31-corpus-contracts-2/artifacts/`) exactly in
`diagnostics` and `chunk-contracts`; those two carry every verdict. The other
three artifacts carry the checkout root and identity digests and have differed
at every re-pin in this chain for that reason, which this tool states rather
than hides.

Usage: python3 measure/c-gateway-return-position/byte_identity.py <run-dir> [archive-dir]
"""
import gzip
import hashlib
import json
import sys
import pathlib

ARTIFACTS = {
    'diagnostics': 'resource-diagnostics.json',
    'source': 'monkey-source.json',
    'chunk-contracts': 'resource-diagnostics.chunk-contracts.json',
    'source-effects': 'resource-diagnostics.source-effects.json',
    'contracts': 'resource-diagnostics.contracts.json',
}
VERDICT_BEARING = ('diagnostics', 'chunk-contracts')


def digest(raw):
    return hashlib.sha256(raw).hexdigest()[:16]


def canonical(raw):
    return hashlib.sha256(
        json.dumps(json.loads(raw), sort_keys=True, separators=(',', ':')).encode()
    ).hexdigest()[:16]


def main():
    run = pathlib.Path(sys.argv[1])
    archive = pathlib.Path(
        sys.argv[2] if len(sys.argv) > 2
        else 'docs/reports/w31-corpus-contracts-2/artifacts'
    )
    print('archive: %s' % archive)
    print('run:     %s' % run)
    print()
    verdicts_identical = True
    for stem, produced in ARTIFACTS.items():
        a = gzip.decompress((archive / f'{stem}.json.gz').read_bytes())
        m = (run / produced).read_bytes()
        raw = 'IDENTICAL' if a == m else 'differs  '
        can = 'IDENTICAL' if canonical(a) == canonical(m) else 'differs  '
        mark = '*' if stem in VERDICT_BEARING else ' '
        print('%s%-16s raw %s  canonical %s   %s / %s'
              % (mark, stem, raw, can, digest(a), digest(m)))
        if stem in VERDICT_BEARING and a != m:
            verdicts_identical = False
    print()
    print('* = verdict-bearing artifact')
    if verdicts_identical:
        print('RESULT: the verdict-bearing artifacts reproduce the baseline '
              'BYTE FOR BYTE.')
        return 0
    print('RESULT: a verdict-bearing artifact DIFFERS. Stop and report this; '
          'the chain is not attributable.')
    return 1


if __name__ == '__main__':
    sys.exit(main())
