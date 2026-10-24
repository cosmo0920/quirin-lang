#!/usr/bin/env python3
"""Publish the compact BASE/TIP comparison of W48-SCHED-CLEANUP-PRODUCTION.

Usage: summarize.py RUNDIR ATTRIBUTION_JSON OUT_JSON

RUNDIR is a merged_job.sh run (base-corpus/, tip-corpus/, identity files);
ATTRIBUTION_JSON is compare.py's full private output. The published record
keeps identities, exits, tallies, every status move, every row and null
record that differs beyond the environment digests, and the outer runtime
projection's changed moves; the full attribution stays private.
"""
import json
import pathlib
import sys


def read(path):
    return pathlib.Path(path).read_text().strip()


def main():
    run, attribution, out = map(pathlib.Path, sys.argv[1:4])
    full = json.loads(attribution.read_text())
    sides = {}
    for side in ('base', 'tip'):
        invocation = json.loads((run / f'{side}-corpus/invocation.json').read_text())
        sides[side] = {
            'revision': read(run / f'{side}-head'),
            'binary_sha256': invocation['binary_sha256'],
            'binary_sha256_after': invocation['binary_sha256_after'],
            'exit': invocation['exit'], 'identity_error': invocation.get('identity_error'),
            'started': invocation['started'], 'ended': invocation['ended'],
            'jobs': invocation['jobs'], 'link_map_sha256': invocation['link_map_sha256'],
            'free_bytes_before': invocation['free_bytes_before'],
            'free_bytes_after': invocation['free_bytes_after'],
            'environment': invocation['environment'],
            'private_artifacts': str(run / f'{side}-corpus'),
        }
    outer = {}
    for side in ('base', 'tip'):
        document = json.loads((run / f'{side}-corpus/corpus-outer.json').read_text())
        outer[side] = {name: {'moves': scenario.get('moves'), 'moved_rows': len(scenario.get('moved', []))}
                       for name, scenario in document['scenarios'].items()}
        outer[side]['static'] = document.get('static')
    tip_outer = json.loads((run / 'tip-corpus/corpus-outer.json').read_text())
    base_outer = json.loads((run / 'base-corpus/corpus-outer.json').read_text())
    added = {}
    for name, scenario in tip_outer['scenarios'].items():
        before = {json.dumps(x, sort_keys=True) for x in base_outer['scenarios'][name]['moved']}
        added[name] = [{'file': x['file'], 'function': x['function'], 'part': x['part'],
                        'reasons': x.get('reasons'), 'runtime': x.get('runtime')}
                       for x in scenario['moved'] if json.dumps(x, sort_keys=True) not in before]
    rows_beyond = full['rows_differing_beyond_environment_digests']
    document = {
        'card': 'W48-SCHED-CLEANUP-PRODUCTION',
        'route': 'conditional eight-home pinned corpus (batch4 link map), run_pinned_audit.py from each side snapshot',
        'sides': sides,
        'custody': {'base': full['base_custody'], 'tip': full['tip_custody'],
                    'rows': [full['base_rows'], full['tip_rows']]},
        'status_moves': full['status_moves'],
        'environment_digest_fields': full['environment_digest_fields'],
        'rows_with_any_byte_change': len(full['status_moves']) + len(full['same_status_changes']),
        'rows_differing_beyond_environment_digests': rows_beyond,
        'null_records': {'base': full['null_projection']['base_records'],
                         'tip': full['null_projection']['tip_records'],
                         'differing_beyond_environment_digests': {
                             'base': [{'function': r['function'], 'custody': r['custody'], 'combined': r['combined']}
                                      for r in full['null_records_differing_beyond_environment_digests']['only_base']],
                             'tip': [{'function': r['function'], 'custody': r['custody'], 'combined': r['combined']}
                                     for r in full['null_records_differing_beyond_environment_digests']['only_tip']]}},
        'outer_runtime_projection': {'summary': outer, 'rows_added_to_moved': added},
        'unsafe_qualification': 'provisional; not an upstream defect claim',
        'private_full_attribution': str(attribution),
    }
    out.write_text(json.dumps(document, indent=1) + '\n')


if __name__ == '__main__':
    main()
