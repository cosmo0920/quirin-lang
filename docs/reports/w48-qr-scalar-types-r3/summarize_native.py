#!/usr/bin/env python3
"""Fold the native summaries card W48-QR-SCALAR-TYPES-R3's tests write.

Usage:

    QCORE_W48_R3_NATIVE_SUMMARY=../DIR cargo test -p qcore-compiler \\
        --test scalar_positions --test scalar_extern_values
    python3 docs/reports/w48-qr-scalar-types-r3/summarize_native.py DIR OUT.json

The tests write one JSON list per native test into DIR (a path relative to
`compiler/`, the tests' working directory): `fields.json` and
`conversions.json` (item 1, the record fields and the conversion statement on
`source.buffer.v1`), `extern-known.json` and `extern-dynamic.json` (item 2,
known and run-time extern values). Every row is one build of one program:
`O0-ubsan` and `O2-ubsan` (`-fsanitize=undefined,integer
-fno-sanitize-recover=all`) and plain `O2`. A row exists only if its build
succeeded under `-Wall -Wextra -Werror -pedantic` and every run matched the
reference (exit status and each stream), so a sanitizer report — which aborts
the run with a nonzero status — cannot be in a row. OUT holds the rows and the
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
    for name in ["fields", "conversions", "extern-known", "extern-dynamic"]:
        rows = json.loads((directory / f"{name}.json").read_text())
        tables[name] = rows
        per_build = {}
        for row in rows:
            entry = per_build.setdefault(row["build"], {"programs": 0})
            entry["programs"] += 1
            for key in ["lines", "conversions", "values", "raises", "exit_86"]:
                if key in row:
                    entry[key] = entry.get(key, 0) + row[key]
            assert row.get("sanitizer_reports", 0) == 0, row
        totals[name] = per_build
    out.write_text(json.dumps({"totals": totals, "rows": tables}, indent=1) + "\n")
    print(json.dumps(totals, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
