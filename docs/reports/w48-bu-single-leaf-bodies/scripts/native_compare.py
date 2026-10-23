#!/usr/bin/env python3
"""Native BASE/TIP comparison for W48-BU-SINGLE-LEAF-BODIES, any route.

Usage: native_compare.py ROUTE BASE_DIR TIP_DIR OUT_JSON

Copied from docs/reports/w48-sched-cleanup-production/scripts/compare.py
(card/w48-sched-cleanup-production) with one change: ROUTE selects the
report (`corpus/resource-diagnostics.json` for corpus, `resource-diagnostics.json`
otherwise) and the `ROUTE-null.jsonl` / `ROUTE-outer.json` projections.

Each DIR is a measure.py output directory. Writes
the custody tallies on both sides, every row whose status, diagnostics or
contract dependencies differ, every null-projection record that differs, and
whether the outer projection documents are equal. Rows are keyed by
(library, unit file, function, row index within that key) so configuration
replays stay distinct.
"""
import collections
import hashlib
import json
import pathlib
import sys


ROUTE = sys.argv[1]


def report(directory):
    name = 'corpus/resource-diagnostics.json' if ROUTE == 'corpus' else 'resource-diagnostics.json'
    return json.loads((pathlib.Path(directory) / name).read_text())


def rows(document):
    keyed = {}
    seen = collections.Counter()
    for unit in document['units']:
        for row in unit['functions']:
            base = (unit.get('library'), unit['file'], row['function'])
            index = seen[base]
            seen[base] += 1
            keyed[base + (index,)] = normalize(row)
    return keyed


def tally(keyed):
    return dict(sorted(collections.Counter(row['status'] for row in keyed.values()).items()))


def leaves(row):
    return sorted(json.dumps({k: v for k, v in leaf.items() if k != 'line'}, sort_keys=True)
                  for leaf in row.get('diagnostics') or [])


EVIDENCE = 'contract-hypothesis-graph.v1:'


def normalize(value):
    """Drop the analyzer build commit from every `evidence_version`: it names
    the binary's source commit, which differs between BASE and TIP by
    construction and is recorded in each side's invocation."""
    if isinstance(value, dict):
        return {k: (EVIDENCE + 'BUILD' if k == 'evidence_version' and isinstance(v, str)
                    and v.startswith(EVIDENCE) else normalize(v)) for k, v in value.items()}
    if isinstance(value, list):
        return [normalize(v) for v in value]
    return value


# Digests derived from the whole contract environment. The TIP environment
# holds more declarations (rules 32 and 34), so every one of these may
# move on every record by construction; the second comparison blanks them to
# show what else differs.
ENVIRONMENT_DIGESTS = {'assumptions', 'build_identity', 'environment_sha256',
                       'resource_contracts_sha256', 'id'}


def blank_environment(value):
    """Blank environment digests; `reading_site_evidence` is ordered by its
    digest ids, so it is compared as a multiset."""
    if isinstance(value, dict):
        blanked = {k: ('ENV' if k in ENVIRONMENT_DIGESTS and isinstance(v, str) else blank_environment(v))
                   for k, v in value.items()}
        if isinstance(blanked.get('reading_site_evidence'), list):
            blanked['reading_site_evidence'] = sorted(blanked['reading_site_evidence'],
                                                      key=lambda v: json.dumps(v, sort_keys=True))
        return blanked
    if isinstance(value, list):
        return [blank_environment(v) for v in value]
    return value


def lines(path):
    path = pathlib.Path(path)
    if not path.exists():
        return None
    return [json.dumps(normalize(json.loads(line)), sort_keys=True)
            for line in path.read_text().splitlines() if line.strip()]


def sha(path):
    path = pathlib.Path(path)
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def main():
    base_dir, tip_dir, out = sys.argv[2:5]
    base, tip = rows(report(base_dir)), rows(report(tip_dir))
    changes = []
    for key in sorted(set(base) | set(tip), key=lambda k: tuple(str(p) for p in k)):
        before, after = base.get(key), tip.get(key)
        if before is None or after is None:
            changes.append({'key': list(key), 'only': 'tip' if before is None else 'base'})
            continue
        if before == after:
            continue
        entry = {'key': list(key), 'status': [before['status'], after['status']]}
        removed = sorted(set(leaves(before)) - set(leaves(after)))
        added = sorted(set(leaves(after)) - set(leaves(before)))
        if removed or added:
            entry['leaves_removed'] = [json.loads(x) for x in removed]
            entry['leaves_added'] = [json.loads(x) for x in added]
        if before.get('contract_dependencies') != after.get('contract_dependencies'):
            entry['contract_dependencies'] = [before.get('contract_dependencies'),
                                              after.get('contract_dependencies')]
        other = sorted(k for k in set(before) | set(after)
                       if k not in ('status', 'diagnostics', 'contract_dependencies')
                       and before.get(k) != after.get(k))
        if other:
            entry['other_fields_changed'] = other
        changes.append(entry)
    null_base = lines(pathlib.Path(base_dir) / f'{ROUTE}-null.jsonl')
    null_tip = lines(pathlib.Path(tip_dir) / f'{ROUTE}-null.jsonl')
    null_diff = None
    if null_base is not None and null_tip is not None:
        cb, ct = collections.Counter(null_base), collections.Counter(null_tip)
        null_diff = {'base_records': len(null_base), 'tip_records': len(null_tip),
                     'only_base': [json.loads(x) for x in sorted((cb - ct).elements())],
                     'only_tip': [json.loads(x) for x in sorted((ct - cb).elements())]}
    env_rows = [list(k) for k in sorted(set(base) & set(tip), key=lambda k: tuple(str(p) for p in k))
                if blank_environment(base[k]) != blank_environment(tip[k])]
    env_null = None
    if null_base is not None and null_tip is not None:
        nb = collections.Counter(json.dumps(blank_environment(json.loads(x)), sort_keys=True) for x in null_base)
        nt = collections.Counter(json.dumps(blank_environment(json.loads(x)), sort_keys=True) for x in null_tip)
        env_null = {'only_base': [json.loads(x) for x in sorted((nb - nt).elements())],
                    'only_tip': [json.loads(x) for x in sorted((nt - nb).elements())]}
    outer = [normalize(json.loads((pathlib.Path(d) / f'{ROUTE}-outer.json').read_text())) for d in (base_dir, tip_dir)]
    document = {
        'environment_digest_fields': sorted(ENVIRONMENT_DIGESTS),
        'rows_differing_beyond_environment_digests': env_rows,
        'null_records_differing_beyond_environment_digests': env_null,
        'outer_equal_beyond_environment_digests': blank_environment(outer[0]) == blank_environment(outer[1]),
        'base_rows': len(base), 'tip_rows': len(tip),
        'base_custody': tally(base), 'tip_custody': tally(tip),
        'status_moves': [c for c in changes if 'status' in c and c['status'][0] != c['status'][1]],
        'same_status_changes': [c for c in changes if 'status' in c and c['status'][0] == c['status'][1]],
        'row_set_changes': [c for c in changes if 'only' in c],
        'null_projection': null_diff,
        'outer_projection_sha256': [sha(pathlib.Path(base_dir) / f'{ROUTE}-outer.json'),
                                    sha(pathlib.Path(tip_dir) / f'{ROUTE}-outer.json')],
        'outer_projection_equal_after_normalization': normalize(json.loads(
            (pathlib.Path(base_dir) / f'{ROUTE}-outer.json').read_text())) == normalize(json.loads(
            (pathlib.Path(tip_dir) / f'{ROUTE}-outer.json').read_text())),
    }
    pathlib.Path(out).write_text(json.dumps(document, indent=1) + '\n')
    print(json.dumps({k: document[k] for k in ('base_rows', 'tip_rows', 'base_custody', 'tip_custody')}))
    print('status moves:', len(document['status_moves']), 'same-status changes:',
          len(document['same_status_changes']), 'row set changes:', len(document['row_set_changes']))
    if null_diff:
        print('null records', null_diff['base_records'], null_diff['tip_records'],
              'only base', len(null_diff['only_base']), 'only tip', len(null_diff['only_tip']))
    print('rows differing beyond environment digests:', len(document['rows_differing_beyond_environment_digests']))
    if env_null is not None:
        print('null records differing beyond environment digests:', len(env_null['only_base']), len(env_null['only_tip']))
    print('outer equal beyond environment digests:', document['outer_equal_beyond_environment_digests'])
    print('outer bytes equal:', document['outer_projection_sha256'][0] == document['outer_projection_sha256'][1],
          'outer equal after normalization:', document['outer_projection_equal_after_normalization'])


if __name__ == '__main__':
    main()
