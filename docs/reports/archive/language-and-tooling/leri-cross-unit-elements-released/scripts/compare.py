#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Quick row comparison of two `quirin-audit current` reports (not the
arbiter): custody totals, status moves and leaf trades, keyed by unit file,
compile-command digest, function and occurrence.

usage: compare.py BEFORE.json AFTER.json [--json OUT]
"""
import hashlib
import json
import sys
from collections import Counter


def rows(path):
    report = json.load(open(path))
    out = {}
    for unit in report['units']:
        command = unit.get('compile_configuration', {}).get('command', '')
        digest = hashlib.sha256(command.encode()).hexdigest()[:12]
        seen = Counter()
        for row in unit.get('functions', []):
            name = row['function']
            seen[name] += 1
            leaves = sorted(
                d.get('description') or json.dumps(d.get('kind'), sort_keys=True)
                for d in row.get('diagnostics', []))
            out[(unit['file'], digest, name, seen[name])] = (row['status'], leaves)
    return out


def totals(table):
    return dict(Counter(status for status, _ in table.values()))


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    result = {
        'before': totals(before), 'after': totals(after),
        'only_before': sorted(map(list, before.keys() - after.keys())),
        'only_after': sorted(map(list, after.keys() - before.keys())),
        'moves': [], 'leaf_trades': [],
    }
    for key in sorted(before.keys() & after.keys()):
        (s0, l0), (s1, l1) = before[key], after[key]
        lost = sorted((Counter(l0) - Counter(l1)).elements())
        gained = sorted((Counter(l1) - Counter(l0)).elements())
        entry = {'row': list(key), 'before': s0, 'after': s1, 'lost': lost, 'gained': gained}
        if s0 != s1:
            result['moves'].append(entry)
        elif lost or gained:
            result['leaf_trades'].append(entry)
    print(json.dumps({k: v for k, v in result.items() if k not in ('moves', 'leaf_trades')}))
    print('moves', len(result['moves']), 'leaf trades', len(result['leaf_trades']))
    for entry in result['moves'] + result['leaf_trades']:
        print(entry['row'], entry['before'], '->', entry['after'],
              'lost', entry['lost'], 'gained', entry['gained'])
    if '--json' in sys.argv:
        json.dump(result, open(sys.argv[sys.argv.index('--json') + 1], 'w'), indent=1)


main()
