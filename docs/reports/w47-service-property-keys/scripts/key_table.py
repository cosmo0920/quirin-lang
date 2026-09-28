#!/usr/bin/env python3
"""Print the derived key table and per-key reach a census reading records.

Usage: key_table.py CENSUS_JSON

Prints `typed.property_keys` of one census reading (the loop_owner_census
example's output): per keyed setter call on Step B's list-head path, each key's
matched entries and reach (`key …`), then the derived table, one row per
element (`KEY | TYPE | FIELD | EFFECT`), or why the keys are not read.
Read-only.
"""

import json
import sys


def main():
    data = json.load(open(sys.argv[1]))
    for site, rows in data.get("typed", {}).get("property_keys", {}).items():
        print(f"## {site}")
        for row in rows:
            print(row)
        print()


if __name__ == "__main__":
    main()
