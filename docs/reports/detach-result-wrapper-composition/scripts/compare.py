#!/usr/bin/env python3
"""BASE/TIP corpus attribution for DETACH-RESULT-WRAPPER-COMPOSITION.

Usage: compare.py BASE_ROUTE_DIR TIP_ROUTE_DIR OUT_JSON

Each route directory is a `measure.py corpus` output (holding
`corpus/resource-diagnostics.json`, `corpus-null.jsonl` and
`corpus-outer.json`). Reports status counts, every changed row in full, the
unit-level fields that differ, the detach declaration's dependent rows and the
byte equality of the null and outer projections. Read-only.
"""
import collections
import hashlib
import json
import pathlib
import sys

DETACH = 'cJSON_DetachItemViaPointer'
WRAPPERS = [
    'cJSON_DetachItemFromArray', 'cJSON_DeleteItemFromArray',
    'cJSON_DetachItemFromObject', 'cJSON_DetachItemFromObjectCaseSensitive',
    'cJSON_DeleteItemFromObject', 'cJSON_DeleteItemFromObjectCaseSensitive',
    DETACH, 'get_array_item', 'cJSON_GetArrayItem',
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def rows(diagnostics):
    out = {}
    for unit in diagnostics['units']:
        for row in unit.get('functions', []):
            out[(unit['library'], unit['file'], row['function'])] = row
    return out


def side(root):
    root = pathlib.Path(root)
    diagnostics = json.loads((root / 'corpus/resource-diagnostics.json').read_text())
    return root, diagnostics, rows(diagnostics)


def main():
    base_root, base, base_rows = side(sys.argv[1])
    tip_root, tip, tip_rows = side(sys.argv[2])
    counts = {name: dict(collections.Counter(r['status'] for r in table.values()))
              for name, table in [('base', base_rows), ('tip', tip_rows)]}
    changed = []
    for key in sorted(set(base_rows) | set(tip_rows)):
        before, after = base_rows.get(key), tip_rows.get(key)
        if before != after:
            changed.append(dict(row='::'.join(key), before=before, after=after,
                                status_changed=(before or {}).get('status') != (after or {}).get('status')))
    units = {}
    for b, t in zip(base['units'], tip['units']):
        assert (b['library'], b['file']) == (t['library'], t['file'])
        keys = sorted(k for k in set(b) | set(t) if k != 'functions' and b.get(k) != t.get(k))
        if keys:
            units[f"{b['library']}/{b['file']}"] = keys
    dependents = {}
    for name, table in [('base', base_rows), ('tip', tip_rows)]:
        dep = [('::'.join(k), r['status']) for k, r in table.items()
               if DETACH in r.get('contract_dependencies', {}).get('declarations', [])]
        dependents[name] = dict(rows=sorted(dep), count=len(dep),
                                safe=sum(1 for _, s in dep if s == 'Safe'))
    wrappers = {}
    for key, row in tip_rows.items():
        if key[1] == 'cJSON.c' and key[2] in WRAPPERS:
            wrappers[key[2]] = dict(before=base_rows.get(key), after=row)
    evidence = {name: next((u.get('contract_body_evidence', {}).get('clauses', {}).get(DETACH)
                            for u in d['units'] if u['file'] == 'cJSON.c'), None)
                for name, d in [('base', base), ('tip', tip)]}
    projections = {}
    for name in ['corpus-null.jsonl', 'corpus-outer.json']:
        projections[name] = dict(base=sha(base_root / name), tip=sha(tip_root / name))
        projections[name]['equal'] = projections[name]['base'] == projections[name]['tip']
    result = dict(counts=counts, diagnostic_counts=dict(base=base['diagnostic_counts'],
                                                        tip=tip['diagnostic_counts']),
                  changed_rows=changed, changed_status=[c['row'] for c in changed if c['status_changed']],
                  unit_field_differences=units, detach_dependents=dependents,
                  detach_clause_evidence=evidence, wrappers=wrappers, projections=projections)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(counts=counts, changed=len(changed),
                          changed_status=result['changed_status'],
                          units=units, dependents={k: (v['count'], v['safe']) for k, v in dependents.items()},
                          projections={k: v['equal'] for k, v in projections.items()}), indent=1))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
