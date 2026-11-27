#!/usr/bin/env python3
"""Card K6 (round 37): list every K6 leaf group of the user's 2026-11-24 report with its origin text.

usage: sites.py REPORT_JSON RECENSUS_JSON OUT_JSONL

A K6 leaf group is an IND/COND/RET group recensus.py joined to an 11-07 census tag
`expr:<statement|cond|return>:direct`. Each output row carries the unit file, function,
status, the -E -P line, the origin (file:line in the prepared tree), the fine census tags,
whether the row is freed alone by K6, and the origin's source text (the origin line and the
next two, joined), read from the prepared tree (read-only).
"""
import gzip
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recensus  # noqa: E402

TREE = pathlib.Path("/media/Data/GitHub/quirin-lang/build/cycle19/clang/"
                    "046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/fluent-bit-clang-source")


def main():
    report = json.load(open(sys.argv[1]))
    summary = json.load(open(sys.argv[2]))
    freed = {tuple(x.split("::")[1:]) for x in summary["freed_alone_now"]}
    old = {}
    for r in recensus.jsonl_gz(recensus.CENSUS / "artifacts/shape-leaves.jsonl.gz"):
        old[(r["file"], r["function"], r["line"], r["family"], r["sub"])] = r
    defined = {f["function"] for u in report["units"] for f in u.get("functions", [])}
    cache = {}

    def text(origin):
        if not origin or ":" not in origin:
            return ""
        path, line = origin.rsplit(":", 1)
        if path not in cache:
            p = TREE / path
            cache[path] = p.read_text(errors="replace").split("\n") if p.exists() else []
        lines = cache[path]
        n = int(line)
        return " ".join(s.strip() for s in lines[n - 1:n + 2])

    out = open(sys.argv[3], "w")
    count = 0
    for unit in report["units"]:
        for fn in unit.get("functions", []):
            seen = set()
            for leaf in fn["diagnostics"]:
                fam, sub = recensus.family(leaf, defined)
                if fam not in ("IND", "COND", "RET"):
                    continue
                key = (unit["file"], fn["function"], leaf["line"], fam, sub)
                if key in seen:
                    continue
                seen.add(key)
                hit = old.get(key)
                if not hit:
                    continue
                coarse = {recensus.coarse(t) for t in hit["tags"]}
                if not coarse & recensus.K6:
                    continue
                out.write(json.dumps({
                    "file": unit["file"], "function": fn["function"], "status": fn["status"],
                    "line": leaf["line"], "family": fam, "sub": sub, "origin": leaf.get("origin"),
                    "tags": hit["tags"], "facts": hit.get("facts"),
                    "freed_alone": (unit["file"], fn["function"]) in freed and fn["status"] == "Unknown",
                    "text": text(leaf.get("origin")),
                }) + "\n")
                count += 1
    print(count, "K6 leaf groups")


if __name__ == "__main__":
    main()
