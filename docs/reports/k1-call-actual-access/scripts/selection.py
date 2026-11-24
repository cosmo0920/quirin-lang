#!/usr/bin/env python3
"""selection.py RECENSUS_JSON OUT.json [--touched] -- K1: a `quirin-audit current
--selection` file (schema 1) naming every non-Monkey unit that holds a row K1
frees alone (--touched: any row carrying a K1 leaf), grouped by the profile
library the run reported it under (AG1's selection.py, read from recensus.py's
rows). Monkey units are analysed only through --monkey-linked and are left out."""
import gzip
import json
import sys

K1 = {"expr:statement:call-arg", "expr:cond:call-arg", "expr:return:call-arg"}
LIBRARY = {"fluent-bit-chunk": ("fluent_bit", "fluent-bit-chunk")}


def main() -> int:
    recensus, out = sys.argv[1:3]
    touched = "--touched" in sys.argv[3:]
    groups = {}
    with gzip.open(recensus + ".rows.jsonl.gz", "rt") as handle:
        for line in handle:
            row = json.loads(line)
            if row["status"] != "Unknown" or not row["items"]:
                continue
            has = any(set(t) & K1 for _, t, _ in row["items"])
            freed = all(set(t) <= K1 for _, t, _ in row["items"])
            if not (freed or (touched and has)):
                continue
            if (row.get("library") or "").startswith("monkey"):
                continue
            library, label = LIBRARY.get(row["library"], (row["library"], None))
            group = groups.setdefault(library, {"library": library, "files": []})
            if label:
                group["label"] = label
            if row["file"] not in group["files"]:
                group["files"].append(row["file"])
    document = {
        "schema_version": 1,
        "name": "k1-call-actual-access",
        "description": "K1 unit-scoped reproduction of the user's 046accf6 current run: "
        "the units holding a K1 call-actual access row",
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
