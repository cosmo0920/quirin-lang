#!/usr/bin/env python3
"""Archive the BASE/TIP corpus reviews into the report's artifacts directory.

Usage: archive.py PRIVATE_ROOT

PRIVATE_ROOT holds `measure-base-2/`, `measure-tip-1/` and the review outputs
(`comparison.json`, `field-review.json`, `site-review.json`,
`projection-review.json`, `base-plan.json`, `tip-plan.json`). Writes compact
JSON and gzip copies with SHA-256/byte manifests. Read-only on its inputs.
"""
import gzip
import hashlib
import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]


def main():
    root = pathlib.Path(sys.argv[1])
    out = REPORT / 'artifacts'
    out.mkdir(exist_ok=True)
    comparison = json.loads((root / 'comparison.json').read_text())
    summary = {k: comparison[k] for k in ['counts', 'diagnostic_counts', 'changed_status',
                                          'detach_dependents', 'detach_clause_evidence',
                                          'projections']}
    summary['changed_rows_raw'] = len(comparison['changed_rows'])
    summary['units_with_unit_field_differences'] = len(comparison['unit_field_differences'])
    summary['cjson_unit_fields'] = comparison['unit_field_differences'].get('librdkafka/cJSON.c')
    (out / 'corpus-summary.json').write_text(json.dumps(summary, indent=1, sort_keys=True) + '\n')
    (out / 'nine-body-review.json').write_text(json.dumps(comparison['wrappers'], indent=1, sort_keys=True) + '\n')
    (out / 'site-review.json').write_text((root / 'site-review.json').read_text())
    # Compact copies; the complete private outputs keep every raw pair.
    fields = json.loads((root / 'field-review.json').read_text())
    compact = {key: (value if not value['substitution'] else
                     dict(substitution=True, places=value['places'], distinct_pairs=value['distinct_pairs']))
               for key, value in fields['keys'].items()}
    (out / 'field-review.json').write_text(json.dumps(dict(keys=compact, semantic_keys=fields['semantic_keys'],
                                                           substitution_keys=fields['substitution_keys']),
                                                      indent=1, sort_keys=True) + '\n')
    projection = json.loads((root / 'projection-review.json').read_text())
    moved = []
    for entry in projection['outer']:
        if entry['path'].endswith('.moved'):
            base = {json.dumps(m, sort_keys=True) for m in entry['base'] or []}
            tip = {json.dumps(m, sort_keys=True) for m in entry['tip'] or []}
            moved.append(dict(path=entry['path'], only_base=[json.loads(m) for m in sorted(base - tip)],
                              only_tip=[json.loads(m) for m in sorted(tip - base)]))
        else:
            moved.append(entry)
    projection['outer'] = moved
    (out / 'projection-review.json').write_text(json.dumps(projection, indent=1, sort_keys=True) + '\n')
    for name in ['base-plan.json', 'tip-plan.json']:
        (out / (name + '.gz')).write_bytes(gzip.compress((root / name).read_bytes(), mtime=0))
    invocations = {}
    for side, path in [('base', 'measure-base-2/corpus/invocation.json'),
                       ('tip', 'measure-tip-1/corpus/invocation.json'),
                       ('base_failed_attempt_1', 'measure-base-1-failed-libevent-pin/corpus/invocation.json')]:
        if (root / path).exists():
            invocations[side] = json.loads((root / path).read_text())
    (out / 'invocations.json').write_text(json.dumps(invocations, indent=1, sort_keys=True) + '\n')
    manifest = {p.name: dict(bytes=p.stat().st_size, sha256=hashlib.sha256(p.read_bytes()).hexdigest())
                for p in sorted(out.iterdir()) if p.name != 'manifest.json'}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=1, sort_keys=True) + '\n')
    print(json.dumps(manifest, indent=1))


if __name__ == '__main__':
    raise SystemExit(main())
