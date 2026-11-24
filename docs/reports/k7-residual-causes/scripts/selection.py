#!/usr/bin/env python3
"""selection.py RECENSUS.json OUT.json [--touched] -- K7: a `quirin-audit
current --selection` file (schema 1, unit_selection.rs) naming every
non-Monkey unit holding a row the K7 bundle frees alone (--touched: every
unit holding a K7 leaf on an Unknown row is not available from recensus.json,
so this flag adds the units of the loose rows only). Monkey units are
analysed through --monkey-linked, not a selection group (AG1's selection.py)."""
import json
import sys

LIBRARY = {"fluent-bit-chunk": ("fluent_bit", "fluent-bit-chunk")}


def main() -> int:
    recensus, out = sys.argv[1:3]
    data = json.load(open(recensus))
    rows = data["freed_rows"] + (data["loose_rows"] if "--touched" in sys.argv[3:] else data["loose_rows"])
    groups = {}
    for row in rows:
        if row["library"] in ("monkey", "monkey-libevent"):
            continue
        library, label = LIBRARY.get(row["library"], (row["library"], None))
        group = groups.setdefault(library, {"library": library, "files": []})
        if label:
            group["label"] = label
        if row["file"] not in group["files"]:
            group["files"].append(row["file"])
    document = {
        "schema_version": 1,
        "name": "k7-residual-causes",
        "description": "K7-RESIDUAL-CAUSES unit-scoped reproduction of the user's 046accf6 "
        "current run: the units holding a row the K7 bundle frees alone",
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
