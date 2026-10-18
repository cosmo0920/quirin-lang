#!/usr/bin/env python3
"""Summarize every aligned static/runtime row from compare.py's complete diff."""
import json
from pathlib import Path
import re
import sys

comparison = json.loads(Path(sys.argv[1]).read_text())
summary = {}
for route, detail in comparison.items():
    scenarios = {}
    for scenario, item in detail['scenarios'].items():
        changed = []
        new = []
        removed = []
        reason_changes = []
        for row in item['rows']:
            before, after = row['before'], row['after']
            if before is None:
                new.append(row['key'])
            elif after is None:
                removed.append(row['key'])
            elif (before['static'], before['runtime']) != (after['static'], after['runtime']):
                changed.append(row)
            else:
                normalize = lambda records: [re.sub(r'@[0-9a-f]{64}', '@CONTRACT-DIGEST', reason) for reason in records]
                if normalize(before.get('reasons', [])) != normalize(after.get('reasons', [])):
                    reason_changes.append(row['key'])
        scenarios[scenario] = {'before_counts': item['before_counts'], 'after_counts': item['after_counts'],
                               'before_moved_rows': item['before_moved_count'], 'after_moved_rows': item['after_moved_count'],
                               'new_runtime_moves': new, 'removed_runtime_moves': removed,
                               'changed_status_pairs': changed, 'changed_reason_facts': reason_changes}
    summary[route] = {'static_before': detail['static_before'], 'static_after': detail['static_after'],
                       'custody_static_statuses_identical': detail['custody_static_statuses_identical'],
                       'null_static_statuses_identical': detail['null_static_statuses_identical'],
                       'null_complete_rows_identical': detail['null_complete_rows_identical'],
                       'scenarios': scenarios}
    assert detail['custody_static_statuses_identical'] and detail['null_static_statuses_identical'], route
Path(sys.argv[2]).write_text(json.dumps(summary, indent=2, sort_keys=True)+'\n')
for route, detail in summary.items():
    print(route, 'static rows identical')
    for scenario, item in detail['scenarios'].items():
        print(scenario, 'new', len(item['new_runtime_moves']), 'removed', len(item['removed_runtime_moves']),
              'changed', len(item['changed_status_pairs']), 'reason-facts', len(item['changed_reason_facts']))
