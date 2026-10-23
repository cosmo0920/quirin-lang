#!/usr/bin/env python3
"""Prediction census for rule 35's literal arm (W48-BU-SINGLE-LEAF-BODIES).

Usage: literal_candidates.py DIAGNOSTICS SOURCE_ROOT OUT_JSON

For every Unknown row of a corpus `resource-diagnostics.json`, read each
refusal leaf's original source line (`origin`, resolved under the library's
source directory) and mark the leaf a literal-dereference candidate when the
line spells `*"` (a `*` applied to a string literal, through any spaces) or one
of librdkafka's two macros that expand to one (`RD_NOTREACHED`, `RD_BUG`). A
row is a candidate to be freed when every leaf is such a candidate and the
leaf's family is the lexical gate's (`indirect memory access ...` or one of the
`unmodeled call, aggregate or indirect access in ...` positions). This is a
census of spellings, not a verdict: a macro line can hold another access, and a
freed leaf can expose a later one. Read-only.
"""
import collections
import json
import pathlib
import re
import sys

ROOTS = {
    'librdkafka': 'lib/librdkafka-2.15.0/src',
    'cmetrics': 'lib/cmetrics/src',
    'monkey-libevent': 'lib/monkey',
    'ctraces': 'lib/ctraces/src',
    'chunkio': 'lib/chunkio/src',
    'fluent-bit-chunk': 'src',
}
LITERAL = re.compile(r'\*\s*"|RD_NOTREACHED\s*\(|RD_BUG\s*\(')
GATE = ('indirect memory access is outside the resource projection',
        'unmodeled call, aggregate or indirect access in ')


def main():
    diagnostics = json.loads(pathlib.Path(sys.argv[1]).read_text())
    source = pathlib.Path(sys.argv[2])
    cache = {}

    def line_text(library, origin):
        if not origin or ':' not in origin:
            return None
        name, number = origin.rsplit(':', 1)
        root = source / ROOTS.get(library, '')
        key = (library, name)
        if key not in cache:
            direct = root / name
            if direct.is_file():
                cache[key] = direct.read_text(errors='replace').split('\n')
            else:
                found = [p for p in source.rglob(pathlib.Path(name).name)
                         if str(p).endswith(name)]
                cache[key] = found[0].read_text(errors='replace').split('\n') if len(found) == 1 else None
        lines = cache[key]
        index = int(number) - 1
        return lines[index] if lines and 0 <= index < len(lines) else None

    rows, leaves = [], collections.Counter()
    for unit in diagnostics['units']:
        for row in unit.get('functions', []):
            if row['status'] != 'Unknown':
                continue
            marks = []
            for leaf in row.get('diagnostics', []):
                text = line_text(unit['library'], leaf.get('origin'))
                literal = bool(text and LITERAL.search(text))
                gate = leaf['description'].startswith(GATE)
                marks.append(literal and gate)
                if literal and gate:
                    leaves[leaf['description']] += 1
            if any(marks):
                rows.append(dict(row=f"{unit['library']}/{unit['file']}::{row['function']}",
                                 literal_leaves=sum(marks), leaves=len(marks),
                                 all_literal=all(marks)))
    result = dict(rows_touched=len(rows), rows_all_literal=sum(r['all_literal'] for r in rows),
                  leaves_by_family=dict(leaves), rows=rows)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'rows'}, indent=1))
    for r in rows:
        if r['all_literal']:
            print(r['row'], r['literal_leaves'])


if __name__ == '__main__':
    main()
