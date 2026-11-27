#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES (`W45-CALLEE-CAST-LEXICAL-PREMISE`): tally the
`cast-representation-callee-census` parity rows of one corpus run.

usage: callee_census.py PARITY_JSONL [--json OUT]

Rows are deduplicated by (function, line, name, clang, lexical): the W4 fixed
point analyses a body more than once. `clang` is Clang's decision (a call
through a parenthesised name, or a cast to a one-identifier type over a
parenthesised operand); `lexical` is what the function's normalised tokens
spell on that line (`call`: W45 rewrote the group to `NAME (`; `cast`: the
group `( NAME ) (` was kept; `both`/`absent`: the line spells both or neither,
which a line-level join cannot decide). Every disagreement is listed.
"""
import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    rows = set()
    with args.parity.open() as stream:
        for line in stream:
            if '"cast-representation-callee-census"' not in line:
                continue
            row = json.loads(line)
            rows.add((row["function"], row["line"], row["name"], row["clang"], row["lexical"]))
    tally = collections.Counter((clang, lexical) for _, _, _, clang, lexical in rows)
    disagreements = sorted(row for row in rows if row[3] != row[4])
    record = {
        "sites": len(rows),
        "by_clang_and_lexical": {f"{clang}/{lexical}": count for (clang, lexical), count in sorted(tally.items())},
        "disagreements": [
            {"function": f, "line": l, "name": n, "clang": c, "lexical": x}
            for f, l, n, c, x in disagreements
        ],
    }
    print(json.dumps({k: v for k, v in record.items() if k != "disagreements"}, indent=1))
    for row in record["disagreements"]:
        print(row)
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
