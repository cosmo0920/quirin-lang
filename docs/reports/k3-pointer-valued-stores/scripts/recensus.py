#!/usr/bin/env python3
"""Card K3 (round 34): re-census the `pointer-valued-store` bundle on the user's
2026-11-24 `current` report.

usage: recensus.py REPORT_JSON OUT_JSON

The 11-07 census (CURRENT-ACCESS-ALIAS-CENSUS, base 7b726587b) tagged every
IND/COND/RET leaf group by (file, function, analysed line, family). The 11-24 run
analysed the same Fluent Bit 046accf6 compile database, so the analysed text and
its line numbers are the same; each 11-24 IND/COND/RET leaf takes the tags of the
11-07 group at its (file, function, line, family). A leaf with no 11-07 group is
`unmapped` (counted, never taken as removable). A row is *freed alone* by K3
exactly as rank.py frees it: every one of its diagnostics is an IND/COND/RET leaf
whose tags are a subset of {pointer-valued-store}. Read-only; no scanner.
"""
import collections
import gzip
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CENSUS = ROOT / "docs/reports/current-access-alias-census"
TAG = "pointer-valued-store"
FAMILY_IND = "indirect memory access is outside the resource projection"
FAMILY_PREFIX = "unmodeled call, aggregate or indirect access in "
CONDITION_KIND = {"branch condition", "loop condition", "for-loop condition", "do-while condition",
                  "switch expression"}


def family(diagnostic):
    if diagnostic["category"] == "unknown_alias":
        return "ALIAS"
    if diagnostic["category"] != "unsupported":
        return diagnostic["category"].upper()
    text = diagnostic["description"]
    if text == FAMILY_IND:
        return "IND"
    if text.startswith(FAMILY_PREFIX):
        rest = text[len(FAMILY_PREFIX):]
        if rest == "return expression":
            return "RET"
        if rest in CONDITION_KIND:
            return "COND"
    if text.startswith("call to uncontracted function"):
        return "UNC"
    return "OTHER"


def frozen_list():
    text = (CENSUS / "predictions/K3.md").read_text()
    block = text.split("## Frozen candidate list", 1)[1].split("```text", 1)[1].split("```", 1)[0]
    return [line.strip() for line in block.splitlines() if line.strip()]


def main():
    report_path, out_path = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    tags_of = collections.defaultdict(set)
    with gzip.open(CENSUS / "artifacts/shape-leaves.jsonl.gz", "rt") as handle:
        handle.readline()
        for line in handle:
            row = json.loads(line)
            tags_of[(row["file"], row["function"], row["line"], row["family"])].update(row["tags"])
    report = json.load(open(report_path))

    leaves = collections.Counter()
    unmapped = collections.Counter()
    touched, freed, freed_if_unmapped = [], [], []
    rows_by_name = collections.defaultdict(list)
    tagged = []
    for index, unit in enumerate(report["units"]):
        for function in unit.get("functions", []):
            key = f"{unit.get('library')}::{unit['file']}::{function['function']}"
            blockers = collections.Counter()
            removable = True
            has_tag = False
            only_unmapped_extra = True
            for d in function["diagnostics"]:
                fam = family(d)
                if fam in ("IND", "COND", "RET"):
                    tags = tags_of.get((unit["file"], function["function"], d["line"], fam))
                    if tags is None:
                        unmapped[fam] += 1
                        blockers["unmapped:" + fam] += 1
                        removable = False
                        continue
                    if TAG in tags:
                        leaves[fam] += 1
                        has_tag = True
                        tagged.append({"file": unit["file"], "function": function["function"],
                                       "line": d["line"], "family": fam, "status": function["status"],
                                       "key": key, "tags": sorted(tags)})
                    if not tags <= {TAG}:
                        removable = False
                        only_unmapped_extra = False
                    blockers[fam + ":" + "+".join(sorted(tags))] += 1
                else:
                    removable = False
                    only_unmapped_extra = False
                    blockers[fam] += 1
            row = {"i": index, "key": key, "status": function["status"],
                   "blockers": dict(blockers)}
            rows_by_name[key].append(row)
            rows_by_name[key.split("::", 1)[1]].append(row)
            if function["status"] != "Unknown" or not has_tag:
                continue
            touched.append(row)
            if removable and function["diagnostics"]:
                freed.append(row)
            elif only_unmapped_extra:
                freed_if_unmapped.append(row)

    frozen = []
    for key in frozen_list():
        # the 11-07 list names Monkey's library `monkey-libevent`; the 11-24 report `monkey`
        rows = rows_by_name.get(key) or rows_by_name.get(key.split("::", 1)[1], [])
        frozen.append({"key": key, "rows": [{"status": r["status"], "blockers": r["blockers"]} for r in rows]})
    out = {
        "report": str(report_path),
        "tag": TAG,
        "leaves": dict(leaves),
        "leaves_total": sum(leaves.values()),
        "unmapped_leaves": dict(unmapped),
        "unknown_rows_touched": len(touched),
        "freed_alone_rows": len(freed),
        "freed_alone_distinct": len({r["key"] for r in freed}),
        "freed_alone": sorted(r["key"] for r in freed),
        "freed_if_unmapped_lines_also_go": sorted(r["key"] for r in freed_if_unmapped),
        "frozen_1107": frozen,
        "tagged_leaves": tagged,
    }
    pathlib.Path(out_path).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: out[k] for k in ("leaves", "leaves_total", "unmapped_leaves", "unknown_rows_touched",
                                           "freed_alone_rows", "freed_alone_distinct")}))


if __name__ == "__main__":
    main()
