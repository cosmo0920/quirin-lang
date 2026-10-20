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
    parser.add_argument('diagnostics', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    raw = args.sidecar.read_bytes()
    data = json.loads(raw)
    declarations = data['contract_dependencies']['declarations']
    diagnostics_raw = args.diagnostics.read_bytes()
    diagnostics = json.loads(diagnostics_raw)
    definition = declarations['__errno_location']['hypothesis_graph']
    assert 'scalar_declaration' in definition['atoms']
    assert definition['premises']['nested_call_admission']['children'] == ['checked_reading_route', 'scalar_declaration', 'untracked_effects']
    potential, unrelated = [], []
    # Preserve whole row records for exact replay, including caller contexts.
    for unit in diagnostics['units']:
        for row in unit['functions']:
            record = row['contract_dependencies']
            item = {'unit': unit['file'], 'function': row['function'],
                    'context': row['hypothesis_context'], 'dependency': record}
            if 'scalar_declaration' in record['hypothesis_dependencies'].get('__errno_location', []):
                potential.append(item)
            else:
                unrelated.append(item)
    assert potential, 'no production row consults the selected atom'
    # One hypothetical refutation applies only in its independently held Context.
    # Other dependent contexts are potential impact, never silently refuted.
    selected_context = potential[0]['context']
    affected = [row for row in potential if row['context'] == selected_context]
    other_contexts = [row for row in potential if row['context'] != selected_context]
    result = {'kind': 'synthetic-refuted-prerequisite-impact-only',
              'source_sidecar': str(args.sidecar.resolve()), 'source_sha256': hashlib.sha256(raw).hexdigest(),
              'diagnostics': str(args.diagnostics.resolve()), 'diagnostics_sha256': hashlib.sha256(diagnostics_raw).hexdigest(),
              'proposition': 'libc.__errno_location.nested_call_admission.scalar_declaration.v1',
              'antecedent': 'same independently held artifact/property/profile/scope/bounds/assumptions/source/build/evidence Context; scalar_declaration evidence disposition Refuted',
              'premise': 'nested_call_admission', 'declaration': '__errno_location',
              'property': 'resource-custody.v1',
              'property_boundary': 'No custody-to-null or static-to-runtime propagation is inferred. Full null/runtime comparison is retained separately.',
              'selected_refutation_context': selected_context, 'affected': affected,
              'dependent_other_contexts_unchanged': other_contexts, 'potential_consulting_rows': len(potential), 'unrelated': unrelated,
              'meaning': 'The dependency map lists all potential direct/summary users. A hypothetical Refuted disposition reaches only exact selected-Context instances; dependent instances in other contexts and unrelated rows remain unchanged. No source mismatch or reachable C violation is asserted; no static, null or runtime verdict is rewritten.'}
    args.output.write_text(json.dumps(result, indent=2)+'\n')


if __name__ == '__main__':
    main()
