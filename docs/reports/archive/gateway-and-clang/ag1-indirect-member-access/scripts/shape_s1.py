#!/usr/bin/env python3
"""shape_s1.py PREMISES.jsonl.gz UNITS_DIR -- AG1: the leaves whose every
path is a store of a call result into a direct member `L->F` of a local
record pointer (`hops == 1`), split by how the local's declaration spells its
type in the analysed text (a `struct TAG *` keyword spelling, a typedef name)
and how the member is declared, with the sole-blocker functions each sub-shape
alone would leave with no access leaf."""
import collections
import gzip
import json
import pathlib
import re
import sys

IND = "indirect memory access is outside the resource projection"


def spelling(text, function, root):
    """The specifier the function's declaration of `root` is written with."""
    start = text.find(function + "(")
    if start < 0:
        return "?"
    body = text[start:start + 20000]
    match = re.search(r"([A-Za-z_][A-Za-z_0-9 ]*?)\s*\*\s*" + re.escape(root) + r"\s*[=;,]", body)
    if not match:
        return "?"
    words = match.group(1).split()
    words = [w for w in words if w not in ("const", "volatile", "register")]
    if len(words) >= 2 and words[-2] in ("struct", "union"):
        return "keyword"
    return "typedef:" + words[-1] if words else "?"


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    units = pathlib.Path(sys.argv[2])
    texts = {}
    records = {json.loads(l)["document"]: json.loads(l)["file"]
               for l in (units / "units.jsonl").read_text().splitlines()}
    by_function = collections.defaultdict(list)
    for row in rows:
        by_function[(row["file"], row["function"])].append(row)
    shapes = collections.Counter()
    sole_leaves = collections.defaultdict(set)
    for row in rows:
        paths = row["paths"]
        stores = [p for p in paths if p["access"] == "store" and p["value"] == "pointer"
                  and p.get("rhs", "").startswith("call:") and p["hops"] == 1
                  and p["root_kind"] == "local"]
        if not stores:
            continue
        store = stores[0]
        name = records.get(row["file"])
        if name not in texts:
            texts[name] = (units / name).read_text(errors="replace") if name else ""
        spelled = spelling(texts[name], row["function"], store["root"])
        member = store["steps"][-1]["type"]
        kind = (spelled.split(":")[0], "plain" if member.count("*") == 1 else "other")
        shapes[kind] += 1
        key = (row["file"], row["function"])
        sole = row["status"] == "Unknown" and set(row["families"]) <= {IND}
        if sole:
            sole_leaves[kind].add(key)
    for kind, count in shapes.most_common():
        print(kind, count, "sole functions touched:", len(sole_leaves[kind]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
