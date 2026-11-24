#!/usr/bin/env python3
"""Card K7-RESIDUAL-CAUSES: re-census bundle K7 of CURRENT-ACCESS-ALIAS-CENSUS
on the user's 2026-11-24 `current` report.

usage: recensus.py REPORT.json OUT.json

The 11-07 census (base 7b726587b) tagged every IND/COND/RET leaf group by the
typed path's refusal causes (artifacts/shape-leaves.jsonl.gz) and every
`unknown_alias` leaf by its W47 discharge group (artifacts/alias-leaves.jsonl.gz).
Both runs read the same prepared Fluent Bit 046accf6 texts, so a leaf's
`(file, function, line, family)` identity is shared between them (checked on
the frozen K7 list: every line agrees). This script re-reads the 11-24 report
and gives each leaf the 11-07 tag of the same identity; a leaf the 11-07 run
did not have is tagged `new:<family>` and is never counted as removable.

A row is freed by a tag set S (rank.py's rule) when every item is an
IND/COND/RET/ALIAS item whose tags are all in S. Figures are upper bounds.
"""
import collections
import gzip
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
CENSUS = HERE.parents[1] / "current-access-alias-census" / "artifacts"

K7 = {"address-of-base", "aggregate-value", "call", "cast", "function-designator", "indirect-call",
      "pointer-arithmetic", "pointer-field-step", "pointer-step", "root-other", "statement-expression",
      "static-local", "union", "volatile-atomic"}
FAMILY_IND = "indirect memory access is outside the resource projection"
FAMILY_PREFIX = "unmodeled call, aggregate or indirect access in "
CONDITION_KIND = {"branch condition": "if", "loop condition": "while", "for-loop condition": "for",
                  "do-while condition": "do", "switch expression": "switch"}
UNC = re.compile(r"call to uncontracted function `([^`]*)`")
CANDIDATE = {"IND", "COND", "RET", "ALIAS"}


def jsonl_gz(path):
    with gzip.open(path, "rt") as handle:
        handle.readline()
        return [json.loads(line) for line in handle]


def family(leaf, defined):
    category, description = leaf["category"], leaf["description"]
    if category == "unknown_alias":
        return "ALIAS", None
    if category != "unsupported":
        return category.upper(), None
    if description == FAMILY_IND:
        return "IND", None
    if description.startswith(FAMILY_PREFIX):
        rest = description[len(FAMILY_PREFIX):]
        if rest == "return expression":
            return "RET", None
        if rest in CONDITION_KIND:
            return "COND", CONDITION_KIND[rest]
    match = UNC.match(description)
    if match:
        return ("UNC-RUN" if match.group(1) in defined else "UNC-EXT"), match.group(1)
    return "OTHER", None


def coarse(tag):
    if not tag.startswith("expr:"):
        return tag
    _, construct, position, _roots = tag.split(":", 3)
    kind = "cond" if construct.startswith("cond") else construct
    return f"expr:{kind}:{'call-arg' if 'call-arg' in position else 'direct'}"


def main():
    report_path, out_path = sys.argv[1], pathlib.Path(sys.argv[2])
    shape = collections.defaultdict(list)
    for row in jsonl_gz(CENSUS / "shape-leaves.jsonl.gz"):
        shape[(row["file"], row["function"], row["line"], row["family"])].append(row)
    alias = collections.defaultdict(list)
    for row in jsonl_gz(CENSUS / "alias-leaves.jsonl.gz"):
        alias[(row["file"], row["function"], row["line"])].append(row)
    old_functions = {}
    with gzip.open(CENSUS / "shape-functions.jsonl.gz", "rt") as handle:
        for line in handle:
            f = json.loads(line)
            old_functions.setdefault((f["file"], f["function"]), []).append(f)

    report = json.load(open(report_path))
    defined = {f["function"] for u in report["units"] for f in u["functions"]}
    rows = []
    for index, unit in enumerate(report["units"]):
        for function in unit["functions"]:
            items, k7_leaves = [], []
            for leaf in function["diagnostics"]:
                fam, sub = family(leaf, defined)
                key = (unit["file"], function["function"], leaf["line"])
                if fam in ("IND", "COND", "RET"):
                    old = shape.get(key + (fam,))
                    tags = {coarse(t) for t in old[0]["tags"]} if old else {"new:" + fam}
                    items.append((fam, tags))
                    if tags & K7:
                        k7_leaves.append({"line": leaf["line"], "origin": leaf.get("origin"), "family": fam,
                                          "tags": sorted(tags), "facts": old[0]["facts"] if old else None})
                elif fam == "ALIAS":
                    old = alias.get(key)
                    items.append((fam, {"alias:" + old[0]["group"]} if old else {"new:ALIAS"}))
                elif fam in ("UNC-RUN", "UNC-EXT"):
                    items.append((fam, {fam + ":" + sub}))
                else:
                    items.append((fam, {"fix:" + fam + ":" + leaf["category"]}))
            rows.append({"i": index, "file": unit["file"], "library": unit.get("library"),
                         "function": function["function"], "status": function["status"], "items": items,
                         "k7": k7_leaves})

    unknown = [r for r in rows if r["status"] == "Unknown"]

    def freed(S, population):
        return [r for r in population if r["items"] and all(f in CANDIDATE and t <= S for f, t in r["items"])]

    leaves = collections.Counter()
    touched = collections.defaultdict(set)
    for n, r in enumerate(unknown):
        for f, t in r["items"]:
            for tag in t & K7:
                leaves[tag] += 1
                touched[tag].add(n)
    per_tag = {t: {"leaves": leaves[t], "rows": len(touched[t]), "alone": len(freed({t}, unknown))}
               for t in sorted(K7)}
    bundle_freed = freed(K7, unknown)
    new_leaves = collections.Counter(next(iter(t)) for r in unknown for f, t in r["items"]
                                     if len(t) == 1 and next(iter(t)).startswith("new:"))
    # Upper bound if every leaf the 11-07 run did not see were also removable.
    loose = [r for r in unknown if r["items"] and all(
        f in CANDIDATE and all(x in K7 or x.startswith("new:") for x in t) for f, t in r["items"])
        and any(t & K7 for f, t in r["items"])]

    # 11-07 frozen list.
    frozen = []
    draft = (CENSUS.parent / "predictions" / "K7.md").read_text().split("## Frozen candidate list")[1]
    for line in draft.splitlines():
        if line.count("::") == 2:
            lib, path, fn = line.strip().split("::")
            hits = [r for r in rows if r["function"] == fn and r["file"] == path]
            frozen.append({"function": line.strip(), "rows": [
                {"status": r["status"], "items": sorted("+".join(sorted(t)) for f, t in r["items"]),
                 "freed_by_k7": bool(freed(K7, [r]))} for r in hits]})

    def describe(r):
        return {"file": r["file"], "function": r["function"], "library": r["library"], "i": r["i"],
                "items": sorted("+".join(sorted(t)) for f, t in r["items"]), "k7": r["k7"]}

    out = {
        "schema": "qcore.k7-residual-recensus.v1",
        "report": report_path,
        "rows": len(rows), "status": dict(collections.Counter(r["status"] for r in rows)),
        "k7_leaves": sum(leaves.values()),
        "k7_rows_touched": len(set().union(*touched.values())) if touched else 0,
        "per_tag": per_tag,
        "bundle_freed_alone": len(bundle_freed),
        "bundle_freed_alone_distinct": len({(r["file"], r["function"]) for r in bundle_freed}),
        "bundle_freed_alone_if_new_leaves_removable": len(loose),
        "new_leaves_on_unknown_rows": dict(new_leaves),
        "freed_rows": [describe(r) for r in bundle_freed],
        "loose_rows": [describe(r) for r in loose if r not in bundle_freed],
        "frozen_11_07": frozen,
    }
    out_path.write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps({k: v for k, v in out.items() if k not in ("freed_rows", "loose_rows", "frozen_11_07")},
                     indent=1))
    print("frozen 11-07:", collections.Counter(
        ("Safe" if all(x["status"] == "Safe" for x in f["rows"]) else
         "freed" if any(x["freed_by_k7"] for x in f["rows"]) else "changed") for f in frozen))


if __name__ == "__main__":
    main()
