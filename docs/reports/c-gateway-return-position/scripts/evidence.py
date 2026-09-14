#!/usr/bin/env python3
"""Archive card C-GATEWAY-RETURN-POSITION's corpus artifacts.

Deterministic gzip (mtime=0, level 9), the same shape every archive in this
chain uses, plus a manifest of raw and compressed digests. The BEFORE
reproduction is re-asserted here rather than trusted from a log: if it ever
stops holding, this refuses to write.

Usage: python3 measure/c-gateway-return-position/evidence.py
"""
import gzip
import hashlib
import json
import pathlib

ROOT = pathlib.Path('.')
RUNS = ROOT / 'measure/c-gateway-return-position/runs'
OUT = ROOT / 'docs/reports/c-gateway-return-position/artifacts'
BASELINE = ROOT / 'docs/reports/w31-corpus-contracts-2/artifacts'

ARTIFACTS = {
    'diagnostics': 'resource-diagnostics.json',
    'source': 'monkey-source.json',
    'chunk-contracts': 'resource-diagnostics.chunk-contracts.json',
    'source-effects': 'resource-diagnostics.source-effects.json',
    'contracts': 'resource-diagnostics.contracts.json',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    # The load-bearing assertion of the whole chain: the BEFORE run reproduces
    # the current baseline's verdict-bearing artifacts byte for byte, so every
    # AFTER delta is attributable to the gateway change and to nothing else.
    for stem in ('diagnostics', 'chunk-contracts'):
        archived = gzip.decompress((BASELINE / f'{stem}.json.gz').read_bytes())
        measured = (RUNS / 'before' / ARTIFACTS[stem]).read_bytes()
        assert archived == measured, (
            f'BEFORE does not reproduce the baseline {stem}; the chain is not '
            f'attributable and nothing may be archived'
        )

    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {
        'schema_version': 1,
        'card': 'C-GATEWAY-RETURN-POSITION',
        'baseline': str(BASELINE),
        'before_reproduces_baseline': ['diagnostics', 'chunk-contracts'],
        'stages': {},
    }
    for stage in ('before', 'after'):
        manifest['stages'][stage] = {}
        for stem, produced in ARTIFACTS.items():
            raw = (RUNS / stage / produced).read_bytes()
            packed = gzip.compress(raw, compresslevel=9, mtime=0)
            if stage == 'after':
                (OUT / f'{stem}.json.gz').write_bytes(packed)
            else:
                (OUT / f'{stem}-before.json.gz').write_bytes(packed)
            manifest['stages'][stage][stem] = {
                'sha256': sha(raw),
                'gzip_sha256': sha(packed),
                'bytes': len(raw),
            }
    (OUT / 'artifact-manifest.json').write_text(
        json.dumps(manifest, indent=1, sort_keys=True) + '\n')

    for stem in ARTIFACTS:
        b = manifest['stages']['before'][stem]['sha256']
        a = manifest['stages']['after'][stem]['sha256']
        print('%-16s %s  %s / %s'
              % (stem, 'IDENTICAL' if a == b else 'differs  ', b[:16], a[:16]))
    print('\nwrote %s' % OUT)


if __name__ == '__main__':
    main()
