#!/usr/bin/env python3
"""Attribute every BASE/TIP difference of the resumed W48-SCHED-CLEANUP-PRODUCTION pair to a rule.

Usage: attribute.py ATTRIBUTION_JSON OUT_JSON

ATTRIBUTION_JSON is compare.py's full output for the pair. BASE 4dd8c8013
holds none of rules 32, 34 and 35; TIP holds all three (main 8991b7b58 merged
into this branch). Each row that differs beyond the environment digests (compare.py's
`rows_differing_beyond_environment_digests`) is attributed by what changed in
it:

* rule 34 (this card): the row rests on, or lost a leaf naming,
  `mk_http_thread_destroy`, or is a Monkey scheduler/http-thread row;
* rule 32 (sibling W48-RK-LOG-CALLER-FRAME): it rests on, or lost a leaf
  naming, `rd_kafka_log0` / `rd_kafka_log_buf`;
* otherwise `unattributed`, listed with its removed and added leaves for a
  reading (rule 35, sibling W48-BU-SINGLE-LEAF-BODIES, lifts leaves without a
  declaration, so its rows land here and are named by reading them).

A row matching both 32 and 34 is reported under both. Null-projection records
are attributed the same way by function name.
"""
import json
import pathlib
import sys

RULE34_NAMES = ('mk_http_thread_destroy',)
RULE34_FILES = ('mk_server/mk_scheduler.c', 'mk_server/mk_http_thread.c')
RULE32_NAMES = ('rd_kafka_log0', 'rd_kafka_log_buf')


def rules_of(key, entry):
    text = json.dumps(entry)
    rules = []
    if any(n in text for n in RULE34_NAMES) or (key[0] == 'monkey-libevent' and key[1] in RULE34_FILES):
        rules.append('34')
    if any(n in text for n in RULE32_NAMES):
        rules.append('32')
    return rules or ['unattributed']


def main():
    attribution, out = sys.argv[1:3]
    full = json.loads(pathlib.Path(attribution).read_text())
    entries = {tuple(c['key']): c for c in full['status_moves'] + full['same_status_changes']}
    result = {'base_custody': full['base_custody'], 'tip_custody': full['tip_custody'],
              'rows': [full['base_rows'], full['tip_rows']], 'row_set_changes': full['row_set_changes'],
              'status_moves': [], 'same_status_changes': [], 'null_records': [], 'by_rule': {}}

    def count(rules, field):
        for rule in rules:
            bucket = result['by_rule'].setdefault(rule, {'status_moves': 0, 'same_status_changes': 0,
                                                         'null_records': 0})
            bucket[field] += 1

    for key in full['rows_differing_beyond_environment_digests']:
        change = entries[tuple(key)]
        rules = rules_of(key, change)
        record = {'key': key, 'status': change['status'], 'rules': rules,
                  'leaves_removed': change.get('leaves_removed', []),
                  'leaves_added': change.get('leaves_added', []),
                  'contract_dependencies': change.get('contract_dependencies'),
                  'other_fields_changed': change.get('other_fields_changed', [])}
        moved = change['status'][0] != change['status'][1]
        result['status_moves' if moved else 'same_status_changes'].append(record)
        count(rules, 'status_moves' if moved else 'same_status_changes')
    moved_keys = {tuple(m['key']) for m in result['status_moves']}
    assert moved_keys == {tuple(m['key']) for m in full['status_moves']}, 'a status move outside the beyond-digest rows'
    nulls = full['null_records_differing_beyond_environment_digests'] or {'only_base': [], 'only_tip': []}
    for side in ('only_base', 'only_tip'):
        for record in nulls[side]:
            key = [record.get('library'), record['file'], record['function']]
            rules = rules_of(key, record)
            result['null_records'].append({'side': side, 'library': record.get('library'), 'file': record['file'],
                                           'function': record['function'], 'custody': record['custody'],
                                           'combined': record['combined'],
                                           'null_verdict': (record.get('null') or {}).get('verdict'),
                                           'rules': rules})
            if side == 'only_tip':
                count(rules, 'null_records')
    pathlib.Path(out).write_text(json.dumps(result, indent=1) + '\n')
    print(json.dumps({k: result[k] for k in ('base_custody', 'tip_custody', 'rows', 'by_rule')}, indent=1))
    for move in result['status_moves']:
        print(move['rules'], move['status'], move['key'][1], move['key'][2])


if __name__ == '__main__':
    main()
