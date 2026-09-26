#!/usr/bin/env python3
"""Print the rows of the singleplex OR functions and of every row that names
them (card W45-REFUTED-CONTRACT-WITHDRAWAL).

Usage: or_rows.py REPORT [NAME ...]
"""
import gzip
import json
import sys

DEFAULT = [
    "flb_output_task_flush",
    "flb_output_task_singleplex_enqueue",
]


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def main():
    report = load(sys.argv[1])
    names = sys.argv[2:] or DEFAULT
    for unit in report["units"]:
        for function in unit["functions"]:
            named = [
                d for d in function["diagnostics"]
                if any(name in d["description"] for name in names)
            ]
            if function["function"] not in names and not named:
                continue
            print(unit["file"], function["function"], function["status"],
                  len(function["diagnostics"]))
            for diagnostic in function["diagnostics"]:
                if diagnostic in named or diagnostic["category"] != "unsupported":
                    print("   ", diagnostic["category"], diagnostic.get("origin"),
                          diagnostic["description"][:200])
            for clause in function.get("user_count_clauses", []):
                print("    clause", clause)


if __name__ == "__main__":
    main()
