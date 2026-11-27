#!/usr/bin/env python3
"""Summarise one census run: site verdict counts per reading and each
premise's derivation (the `typed.premises` block this card adds).

Usage: summary.py CENSUS_DIR [--premises]
Read-only.
"""

import json
import pathlib
import sys
from collections import Counter

READINGS = ["fixed-configured", "previous-configured", "fixed", "previous",
            "fixed-configured-control", "previous-configured-control"]


def main():
    out = pathlib.Path(sys.argv[1])
    for name in READINGS:
        path = out / f"{name}.json"
        if not path.exists():
            continue
        data = json.loads(path.read_text())
        counts = Counter(site["result"]["verdict"] for site in data["sites"])
        print(f"{name}: discharged {counts['discharged']} / not {counts['not-discharged']} "
              f"/ unresolved {counts['unresolved']}; edges {data['edges']}, "
              f"functions {data['functions']}, seeds {data['unresolved_seeds']}")
        if "--premises" in sys.argv and name == "fixed-configured":
            for key, lines in data.get("typed", {}).get("premises", {}).items():
                print(f"  == {key}")
                for line in lines:
                    if key == "P-PLUGIN-DIR registrations" and ": derived" in line:
                        continue
                    print(f"     {line}")
                if key == "P-PLUGIN-DIR registrations":
                    derived = sum(1 for line in lines if ": derived" in line)
                    print(f"     ({derived} of {len(lines)} registrants derived)")


if __name__ == "__main__":
    main()
