#!/usr/bin/env python3
"""CG26-VARIADICS: the `va-list-statement` and `variadic-tail-statement`
parity rows of one run (`QCORE_TYPED_PATH_PARITY`), de-duplicated by
(function, line, texts, position).

Usage: tail_shapes.py PARITY.jsonl OUT.json
"""

import collections
import json
import pathlib
import sys


def main():
    seen = set()
    va = collections.Counter()
    va_refusals = collections.Counter()
    va_functions = collections.defaultdict(set)
    tails = collections.Counter()
    tail_rows = []
    for line in open(sys.argv[1], encoding="utf-8"):
        if '"va-list-statement"' not in line and '"variadic-tail-statement"' not in line:
            continue
        row = json.loads(line)
        key = (row["function"], row["line"], tuple(row["texts"]), row["position"])
        if key in seen:
            continue
        seen.add(key)
        if row["position"] == "va-list-statement":
            va[(row.get("form"), row["new_admitted"])] += 1
            va_functions[row["new_admitted"]].add(row["function"])
            if not row["new_admitted"]:
                va_refusals[row["new_refusal"]] += 1
        else:
            tails[(row["callee"], row["promoted_scalar_or_pointer"], row["new_admitted"])] += 1
            tail_rows.append(
                {
                    "function": row["function"],
                    "callee": row["callee"],
                    "texts": " ".join(row["texts"]),
                    "promoted": row["promoted_scalar_or_pointer"],
                    "admitted": row["new_admitted"],
                }
            )
    out = {
        "va_statements": {f"{form} admitted={ok}": n for (form, ok), n in sorted(va.items(), key=str)},
        "va_functions": {str(ok): len(v) for ok, v in va_functions.items()},
        "va_refusals": dict(va_refusals.most_common()),
        "tail_statements": {
            f"{callee} promoted={p} admitted={ok}": n for (callee, p, ok), n in sorted(tails.items(), key=str)
        },
        "tail_rows": tail_rows,
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: v for k, v in out.items() if k != "tail_rows"}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
