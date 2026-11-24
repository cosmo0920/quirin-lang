#!/usr/bin/env python3
"""Card K1: list the K1 leaves of the rows K1 frees alone, with origin and source text.

usage: sites.py REPORT_JSON RECENSUS_JSON SOURCE_ROOT OUT_JSONL [--all-touched]

SOURCE_ROOT is the prepared Fluent Bit tree the report was made from (read only).
Each output row: function, preprocessed line, origin, K1 tag, source line text.
"""
import gzip
import json
import pathlib
import sys

K1 = {"expr:statement:call-arg", "expr:cond:call-arg", "expr:return:call-arg"}


def main():
    report_path, recensus_path, source_root, out_path = sys.argv[1:5]
    all_touched = "--all-touched" in sys.argv
    summary = json.load(open(recensus_path))
    rows = []
    with gzip.open(recensus_path + ".rows.jsonl.gz", "rt") as handle:
        for line in handle:
            rows.append(json.loads(line))
    wanted = {}
    for r in rows:
        if r["status"] != "Unknown" or not r["items"]:
            continue
        k1_lines = {line for _, t, line in r["items"] if set(t) & K1}
        if not k1_lines:
            continue
        freed = all(set(t) <= K1 for _, t, _ in r["items"])
        if freed or all_touched:
            wanted[(r["i"], r["function"])] = (k1_lines, freed,
                                               {t for _, ts, line in r["items"] for t in ts if line in k1_lines})
    report = json.load(open(report_path))
    root = pathlib.Path(source_root)
    cache = {}
    out = open(out_path, "w")
    for index, unit in enumerate(report["units"]):
        for fn in unit.get("functions", []):
            key = (index, fn["function"])
            if key not in wanted:
                continue
            k1_lines, freed, tags = wanted[key]
            seen = set()
            for leaf in fn["diagnostics"]:
                if leaf["line"] not in k1_lines or leaf["line"] in seen:
                    continue
                if leaf["category"] != "unsupported":
                    continue
                seen.add(leaf["line"])
                origin = leaf.get("origin") or ""
                text = ""
                if ":" in origin:
                    file, _, number = origin.rpartition(":")
                    path = root / file
                    if path not in cache:
                        try:
                            cache[path] = path.read_text(errors="replace").splitlines()
                        except OSError:
                            cache[path] = []
                    lines = cache[path]
                    if number.isdigit() and 0 < int(number) <= len(lines):
                        text = lines[int(number) - 1].strip()
                out.write(json.dumps({"unit": index, "file": unit["file"], "function": fn["function"],
                                      "freed_alone": freed, "line": leaf["line"], "origin": origin,
                                      "description": leaf["description"], "text": text}) + "\n")
    out.close()


if __name__ == "__main__":
    main()
