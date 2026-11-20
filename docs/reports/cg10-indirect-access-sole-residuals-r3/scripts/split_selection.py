#!/usr/bin/env python3
"""split_selection.py SELECTION.json N OUT_PREFIX -- CG10 r3: split a
`--selection` file (schema 1) into N files, unit i to part i mod N, so the
census-feature runs (one job each: census streams force one job) can run in
parallel processes, each with its own streams."""
import json
import sys


def main() -> int:
    source, count, prefix = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    document = json.load(open(source))
    items = [(g["library"], g.get("label"), f) for g in document["groups"] for f in g["files"]]
    for part in range(count):
        groups = {}
        for library, label, file in items[part::count]:
            group = groups.setdefault(library, {"library": library, "files": []})
            if label:
                group["label"] = label
            group["files"].append(file)
        out = dict(document)
        out["name"] = f"{document['name']}-part{part}"
        out["groups"] = [groups[key] for key in sorted(groups)]
        with open(f"{prefix}-{part}.json", "w") as handle:
            json.dump(out, handle, indent=2)
            handle.write("\n")
    print(len(items), "units in", count, "parts")
    return 0


if __name__ == "__main__":
    sys.exit(main())
