#!/usr/bin/env python3
"""BASE/TIP attribution for W48-BU-SINGLE-LEAF-BODIES (gateway rule 35).

Usage: compare.py ROUTE BASE_ROUTE_DIR TIP_ROUTE_DIR SOURCE_ROOT OUT_JSON

Each route directory is a `measure.py` output (`corpus/resource-diagnostics.json`
for the corpus route, `resource-diagnostics.json` otherwise, plus the route's
null and outer projections). Reports status counts, every row whose content
changed (before and after in full), each status change with its leaf trade,
the twelve brief bodies on both sides, every refusal leaf the two sides do not
share classified by the rule-35 arm its source line spells (a `*"` literal or a
decayed array member is only knowable from the source line, so the arm is a
spelling attribution), unit-level field differences and projection equality.
Read-only.
"""
import collections
import hashlib
import json
import pathlib
import re
import sys

TWELVE = [
    'mk_iov_free_marked', 'rd_kafka_topic_partition_list_cmp',
    'rd_kafka_MemberDescription_copy', 'rd_kafka_group_result_copy',
    'rd_kafka_topic_partition_list_grow', 'rd_kafka_queue_get_sasl', 'rd_map_init',
    'XXH32_digest_endian', 'XXH64_digest_endian', 'rd_kafka_broker_ApiVersion_at_least',
    'rd_kafka_mock_cluster_io_set_event', 'thrd_setname',
]
ROOTS = {
    'librdkafka': 'lib/librdkafka-2.15.0/src',
    'cmetrics': 'lib/cmetrics/src',
    'monkey-libevent': 'lib/monkey',
    'ctraces': 'lib/ctraces/src',
    'chunkio': 'lib/chunkio/src',
    'fluent-bit-chunk': 'src',
}
LITERAL = re.compile(r'\*\s*"|RD_NOTREACHED\s*\(|RD_BUG\s*\(')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def diagnostics_path(root, route):
    return root / ('corpus/resource-diagnostics.json' if route == 'corpus'
                   else 'resource-diagnostics.json')


def rows(diagnostics):
    out = {}
    for unit in diagnostics['units']:
        for row in unit.get('functions', []):
            out[(unit['library'], unit['file'], row['function'])] = row
    return out


def leaf_key(leaf):
    return (leaf.get('category'), leaf.get('description'), leaf.get('origin'))


def main():
    route = sys.argv[1]
    base_root, tip_root = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    source = pathlib.Path(sys.argv[4])
    base = json.loads(diagnostics_path(base_root, route).read_text())
    tip = json.loads(diagnostics_path(tip_root, route).read_text())
    base_rows, tip_rows = rows(base), rows(tip)
    cache = {}

    def line_text(library, origin):
        if not origin or ':' not in origin:
            return None
        name, number = origin.rsplit(':', 1)
        key = (library, name)
        if key not in cache:
            candidates = [source / ROOTS.get(library, '') / name, source / name]
            found = next((c for c in candidates if c.is_file()), None)
            cache[key] = found.read_text(errors='replace').split('\n') if found else None
        lines = cache[key]
        index = int(number) - 1
        return lines[index] if lines and 0 <= index < len(lines) else None

    counts = {name: dict(collections.Counter(r['status'] for r in table.values()))
              for name, table in [('base', base_rows), ('tip', tip_rows)]}
    changed, status_changes, leaf_trade = [], [], collections.Counter()
    for key in sorted(set(base_rows) | set(tip_rows)):
        before, after = base_rows.get(key), tip_rows.get(key)
        if before == after:
            continue
        name = '::'.join(key)
        changed.append(dict(row=name, before=before, after=after))
        b_leaves = collections.Counter(leaf_key(l) for l in (before or {}).get('diagnostics', []))
        a_leaves = collections.Counter(leaf_key(l) for l in (after or {}).get('diagnostics', []))
        removed, added = b_leaves - a_leaves, a_leaves - b_leaves
        for leaf, n in removed.items():
            text = line_text(key[0], leaf[2]) or ''
            arm = 'literal' if LITERAL.search(text) else 'other-line'
            leaf_trade[('removed', leaf[1], arm)] += n
        for leaf, n in added.items():
            leaf_trade[('added', leaf[1], '')] += n
        if (before or {}).get('status') != (after or {}).get('status'):
            status_changes.append(dict(
                row=name, before=(before or {}).get('status'), after=(after or {}).get('status'),
                removed_leaves=[dict(category=l[0], description=l[1], origin=l[2],
                                     source_line=(line_text(key[0], l[2]) or '').strip(), count=n)
                                for l, n in sorted(removed.items())],
                added_leaves=[dict(category=l[0], description=l[1], origin=l[2], count=n)
                              for l, n in sorted(added.items())],
                tip_dependencies=(after or {}).get('contract_dependencies')))
    units = {}
    for b, t in zip(base['units'], tip['units']):
        assert (b['library'], b['file']) == (t['library'], t['file'])
        keys = sorted(k for k in set(b) | set(t) if k != 'functions' and b.get(k) != t.get(k))
        if keys:
            units[f"{b['library']}/{b['file']}"] = keys
    twelve = {}
    for key, row in base_rows.items():
        if key[2] in TWELVE:
            after = tip_rows.get(key)
            twelve['::'.join(key)] = dict(
                before=dict(status=row['status'], diagnostics=row.get('diagnostics', [])),
                after=dict(status=(after or {}).get('status'),
                           diagnostics=(after or {}).get('diagnostics', [])))
    projections = {}
    for name in [f'{route}-null.jsonl', f'{route}-outer.json']:
        projections[name] = dict(base=sha(base_root / name), tip=sha(tip_root / name))
        projections[name]['equal'] = projections[name]['base'] == projections[name]['tip']
    trade = [dict(side=k[0], description=k[1], arm=k[2], count=v) for k, v in sorted(leaf_trade.items())]
    result = dict(route=route, counts=counts,
                  diagnostic_counts=dict(base=base.get('diagnostic_counts'), tip=tip.get('diagnostic_counts')),
                  changed_rows=changed, status_changes=status_changes, leaf_trade=trade,
                  unit_field_differences=units, twelve=twelve, projections=projections)
    pathlib.Path(sys.argv[5]).write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')
    print(json.dumps(dict(counts=counts, changed_rows=len(changed),
                          status_changes=[(s['row'], s['before'], s['after']) for s in status_changes],
                          leaf_trade=trade, units=len(units),
                          projections={k: v['equal'] for k, v in projections.items()}), indent=1))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
