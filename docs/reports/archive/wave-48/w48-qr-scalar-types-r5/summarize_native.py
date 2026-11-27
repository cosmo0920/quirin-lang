#!/usr/bin/env python3
"""Fold the native summaries card W48-QR-SCALAR-TYPES-R5's tests write.

Usage:

    QCORE_W48_R5_NATIVE_SUMMARY=../DIR cargo test -p qcore-compiler \\
        --test scalar_loop_bounds --test scalar_buffer_values
    python3 docs/reports/w48-qr-scalar-types-r5/summarize_native.py DIR OUT.json

The tests write one JSON list per test into DIR (a path relative to
`compiler/`, the tests' working directory): `loops.json` (item 1, every integer
type's loop), `values.json` (item 2, every admitted value statement, comparison
and negation per type, and an index-arithmetic loop per type) and
`checker-table.json` (item 2, the counts of the checker's boundary table:
admitted values and refused faults). Every native row is one build of one
program: `O0-ubsan` and `O2-ubsan` (`-fsanitize=undefined,integer
-fno-sanitize-recover=all`) and plain `O2`. A row exists only if its build
succeeded under `-Wall -Wextra -Werror -pedantic` and the run exited 0 with
each stream equal to the reference, so a sanitizer report — which aborts the
run with a nonzero status — cannot be in a row. OUT holds the rows and the
per-build totals.
"""

import json
import pathlib
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    directory = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    tables = {}
    totals = {}
    for name in ["loops", "values"]:
        rows = json.loads((directory / f"{name}.json").read_text())
        tables[name] = rows
        per_build = {}
        for row in rows:
            entry = per_build.setdefault(row["build"], {"programs": 0})
            entry["programs"] += 1
            for key in ["lines", "statements"]:
                if key in row:
                    entry[key] = entry.get(key, 0) + row[key]
            assert row.get("sanitizer_reports", 0) == 0, row
            assert row.get("exit", 0) == 0, row
        totals[name] = per_build
    checker = json.loads((directory / "checker-table.json").read_text())
    totals["checker-table"] = checker[0]
    out.write_text(json.dumps({"totals": totals, "rows": tables}, indent=1) + "\n")
    print(json.dumps(totals, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
