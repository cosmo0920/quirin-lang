#!/usr/bin/env python3
"""Publish the compact BASE / arms-off / TIP comparison of W48-BU-SINGLE-LEAF-BODIES.

Usage: summarize.py RUN_DIR CMP_DIR OUT_JSON

RUN_DIR is run 5 (`{base,tip,tipoff}-{corpus,system,slice}` measure.py outputs);
CMP_DIR holds compare.py (`ROUTE-A-B.json`), native_compare.py
(`native-ROUTE-A-B.json`) and planner (`plan-SIDE.json`) outputs. TIP against
arms-off isolates rule 35; BASE against arms-off is the sibling rules 32 and
34. The full attributions stay private.
"""
import json
import pathlib
import sys

PAIRS = [('tipoff', 'tip', 'rule 35'), ('base', 'tipoff', 'sibling rules 32 and 34'), ('base', 'tip', 'all')]


def main():
    run, cmp, out = map(pathlib.Path, sys.argv[1:4])
    sides = {}
    for side in ('base', 'tipoff', 'tip'):
        sides[side] = {}
        for route in ('corpus', 'system', 'slice'):
            inv = json.loads((run / f'{side}-{route}/invocation.json').read_text())
            sides[side][route] = {k: inv.get(k) for k in (
                'analyzer_source', 'binary_sha256', 'binary_sha256_after', 'exit', 'identity_error',
                'started', 'ended', 'jobs', 'link_map_sha256', 'report_driver_sha256')}
    routes = {}
    for route in ('corpus', 'system', 'slice'):
        routes[route] = {}
        for a, b, label in PAIRS:
            native = json.loads((cmp / f'native-{route}-{a}-{b}.json').read_text())
            leaves = json.loads((cmp / f'{route}-{a}-{b}.json').read_text())
            outer_a = json.loads((run / f'{a}-{route}/{route}-outer.json').read_text())
            outer_b = json.loads((run / f'{b}-{route}/{route}-outer.json').read_text())
            entry = {
                'attribution': label,
                'custody': [native['base_custody'], native['tip_custody']],
                'static_combined': [outer_a['static']['combined'], outer_b['static']['combined']],
                'static_null': [outer_a['static']['null'], outer_b['static']['null']],
                'status_moves': [{'row': m['key'][:3], 'status': m['status'],
                                  'leaves_removed': [{k: l.get(k) for k in ('description', 'origin')}
                                                     for l in m.get('leaves_removed', [])],
                                  'leaves_added': [{k: l.get(k) for k in ('description', 'origin')}
                                                   for l in m.get('leaves_added', [])]}
                                 for m in native['status_moves']],
                'rows_differing_beyond_environment_digests': len(native['rows_differing_beyond_environment_digests']),
                'null_records_differing_beyond_environment_digests': [
                    {'function': r['function'], 'custody': r.get('custody'), 'combined': r.get('combined'),
                     'null_verdict': (r.get('null') or {}).get('verdict')}
                    for r in native['null_records_differing_beyond_environment_digests']['only_tip']],
                'leaf_trade': leaves['leaf_trade'],
                'outer_rows_added_to_moved': {
                    name: sorted({(x['file'], x['function'], x.get('part')) for x in sc.get('moved', [])}
                                 - {(x['file'], x['function'], x.get('part'))
                                    for x in outer_a['scenarios'][name].get('moved', [])})
                    for name, sc in outer_b['scenarios'].items()},
            }
            if route == 'corpus' and (a, b) == ('tipoff', 'tip'):
                entry['twelve_brief_bodies'] = {
                    k: {'before': v['before']['status'], 'after': v['after']['status'],
                        'after_leaves': [{kk: l.get(kk) for kk in ('description', 'origin')}
                                         for l in v['after']['diagnostics']]}
                    for k, v in sorted(leaves['twelve'].items())}
            routes[route][f'{a}->{b}'] = entry
    planner = {}
    for side in ('base', 'tipoff', 'tip'):
        plan = json.loads((cmp / f'plan-{side}.json').read_text())
        planner[side] = {k: plan[k] for k in ('bodies', 'ceiling', 'baseline_same', 'baseline_cross',
                                              'ceiling_waits_on_bodies')}
        planner[side]['positive_bodies'] = sum(1 for r in plan['rows'] if r.get('score', 0) > 0)
    document = {
        'card': 'W48-BU-SINGLE-LEAF-BODIES',
        'rule': 'gateway 35',
        'route': ('conditional eight-home pinned corpus (batch4 link map) plus configured system and '
                  'tail-pause slice, each through the re-spelled inputs mirror (mirror_inputs.py)'),
        'sides': {'base': '4dd8c80135f8c35d56f64bcce9305b0b081ef9fd',
                  'tipoff': '8879c2d8984fd677c6561bfd9375bea42dbe4f8e with DECAY_ARM and LITERAL_ARM false',
                  'tip': '8879c2d8984fd677c6561bfd9375bea42dbe4f8e'},
        'invocations': sides,
        'routes': routes,
        'planner': planner,
        'unsafe_qualification': 'provisional; not an upstream defect claim',
    }
    out.write_text(json.dumps(document, indent=1) + '\n')


if __name__ == '__main__':
    main()
