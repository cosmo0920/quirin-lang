#!/usr/bin/env python3
"""residuals.py PARITY_JSONL -- card CG14-AGGREGATE-VALUES-AND-INITIALIZERS:
the sizes of the residual classes the card registers, from one run's
`aggregate-value` rows (first row per (function, line, texts, form)):
statements and functions per refusal code, restricted to aggregate
statements (a record or array class, a compound literal, or a string
initializer; a scalar or pointer statement the fallback merely consulted is
not counted).
"""

import collections
import json
import sys

NOT_AGGREGATE = {"declaration:not a record", "declaration:not a record or array"}


def main() -> int:
    first = {}
    with open(sys.argv[1], encoding="utf-8") as stream:
        for line in stream:
            if '"aggregate-value"' not in line:
                continue
            row = json.loads(line)
            if row.get("position") != "aggregate-value":
                continue
            key = (row["function"], row["line"], tuple(row["texts"]), row["form"])
            first.setdefault(key, row)
    statements = collections.Counter()
    functions = collections.defaultdict(set)
    for (function, _, _, form), row in first.items():
        if row["admitted"] or row["refusal"] in NOT_AGGREGATE:
            continue
        code = f"{form} {row['refusal']}"
        statements[code] += 1
        functions[code].add(function)
    record = {
        code: {"statements": count, "functions": len(functions[code])}
        for code, count in statements.most_common()
    }
    print(json.dumps(record, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
