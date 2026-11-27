#!/usr/bin/env python3
"""selection.py ROWS.json OUT.json [--all] -- AG1: a `quirin-audit current
--selection` file (schema 1, unit_selection.rs) naming every non-Monkey unit
that holds a sole-blocker row (--all: any access row), grouped by the
profile library the run reported it under. Monkey units are analysed through
--monkey-linked, not a selection group, so they are left out (their leaves
keep the AST census only)."""
import json
import sys

LIBRARY = {"fluent-bit-chunk": ("fluent_bit", "fluent-bit-chunk")}


def main() -> int:
    rows_path, out = sys.argv[1:3]
    every = "--all" in sys.argv[3:]
    rows = json.load(open(rows_path))["rows"]
    groups = {}
    for row in rows:
        if not (every or row["sole"]) or row["library"] == "monkey":
            continue
        library, label = LIBRARY.get(row["library"], (row["library"], None))
        group = groups.setdefault(library, {"library": library, "files": []})
        if label:
            group["label"] = label
        if row["unit"] not in group["files"]:
            group["files"].append(row["unit"])
    document = {
        "schema_version": 1,
        "name": "ag1-indirect-member-access",
        "description": "AG1-INDIRECT-MEMBER-ACCESS unit-scoped reproduction of the "
        "user's 046accf6 current run: the units holding an access-refusal row",
        "groups": [groups[key] for key in sorted(groups)],
    }
    for group in document["groups"]:
        group["files"].sort()
    with open(out, "w") as handle:
        json.dump(document, handle, indent=2)
        handle.write("\n")
    print({g["library"]: len(g["files"]) for g in document["groups"]})
    return 0


if __name__ == "__main__":
    sys.exit(main())
