#!/usr/bin/env python3
"""Trace a synthetic refuted libc prerequisite through published consultation closures.
No verdict is invalidated or changed by this evidence-only report.
"""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('sidecar', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    raw = args.sidecar.read_bytes()
    data = json.loads(raw)
    declarations = data['contract_dependencies']['declarations']
    dependencies = data['contract_dependencies']['hypothesis_dependencies']
    definition = declarations['__errno_location']['hypothesis_graph']
    assert 'scalar_declaration' in definition['atoms']
    assert definition['premises']['nested_call_admission']['children'] == ['checked_reading_route', 'scalar_declaration', 'untracked_effects']
    affected, unrelated = [], []
    # Preserve whole row records for exact replay, including caller contexts.
    for row, record in sorted(dependencies.items()):
        if 'scalar_declaration' in record.get('__errno_location', []):
            affected.append({'row': row, 'dependency': record})
        else:
            unrelated.append({'row': row, 'dependency': record})
    result = {'kind': 'synthetic-refuted-prerequisite-impact-only',
              'source_sidecar': str(args.sidecar.resolve()), 'source_sha256': hashlib.sha256(raw).hexdigest(),
              'proposition': 'libc.__errno_location.nested_call_admission.scalar_declaration.v1',
              'antecedent': 'same independently held artifact/property/profile/scope/bounds/assumptions/source/build/evidence Context; scalar_declaration evidence disposition Refuted',
              'premise': 'nested_call_admission', 'declaration': '__errno_location',
              'affected': affected, 'unrelated': unrelated,
              'meaning': 'A named premise error reaches exactly consulting evidence records including summary inheritance. No source mismatch or reachable C violation is asserted; no static, null or runtime verdict is rewritten.'}
    args.output.write_text(json.dumps(result, indent=2)+'\n')


if __name__ == '__main__':
    main()
