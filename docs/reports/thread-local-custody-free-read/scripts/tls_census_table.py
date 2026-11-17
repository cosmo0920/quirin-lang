#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Render tls_census.json (from tls_census.py) as the report's census table.

Each row is one reported function (one per compile configuration) that reads
a thread-local object.  `attributable` says whether every diagnostic of the
row is an unresolved identity of a thread-local object or of one of its
copies, the only diagnostics rule 54 can remove.

Usage: tls_census_table.py <tls_census.json>
"""

import json
import re
import sys


def main():
    census = json.load(open(sys.argv[1]))
    for route, rows in census.items():
        errors = [row for row in rows if "error" in row]
        functions = [row for row in rows if "function" in row]
        print(f"### {route}: {len(functions)} rows, {len(errors)} unit errors")
        for error in errors:
            print(f"- unit error {error['unit']}: {error['error'][:120]}")
        print()
        print("| Unit | Function | Status | Thread-local uses | Copies | Reading | Diagnostics | Only thread-local aliases |")
        print("|---|---|---|---|---|---|---|---|")
        for row in functions:
            names = set(row["thread_locals"]) | set(row["copies"])
            alias = re.compile(r"unresolved resource identity `([^`]*)`")
            attributable = all(
                (m := alias.fullmatch(d or "")) and m.group(1) in names
                for d in row["diagnostics"]
            )
            uses = "; ".join(
                f"`{name}`: {', '.join(record['uses'])}"
                for name, record in sorted(row["thread_locals"].items())
            )
            copies = "; ".join(
                f"`{name}`: {', '.join(row['copy_uses'][name])}" for name in row["copies"]
            )
            reading = "admitted" if row["admitted"] else "withheld: " + "; ".join(row["withheld_because"][:3])
            print(
                f"| `{row['unit']}` | `{row['function']}` | {row['status']} | {uses} | {copies or '-'} "
                f"| {reading} | {len(row['diagnostics'])} | {'yes' if attributable and row['diagnostics'] else 'no'} |"
            )
        print()


if __name__ == "__main__":
    main()
