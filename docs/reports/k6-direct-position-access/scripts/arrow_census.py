#!/usr/bin/env python3
"""Card K6: the K6 leaf groups whose -E -P line spells `( & NAME -> ... ) ->`.

usage: arrow_census.py SITES_JSONL PP_DIR OUT_JSON

SITES_JSONL is sites.py's output (non-Monkey rows), PP_DIR preprocess.py's output
(K1's script, the gateway's plain `-E -P` preprocessing). A site is counted when the
leaf's -E -P line holds a group `( & NAME` followed by `->`, member selections or
subscripts, `)` and `->` (rule 81's lexical shape, read on whitespace-split tokens
after separating punctuation), and split by family, freed-alone status and the
group's leading position (return, condition, statement).
"""
import collections
import json
import re
import sys

GROUP = re.compile(r"\(\s*&\s*[A-Za-z_]\w*\s*->\s*[A-Za-z_]\w*(?:\s*(?:->|\.)\s*[A-Za-z_]\w*|\s*\[[^\[\]]*\])*\s*\)\s*->")
OUTER_ADDRESS = re.compile(r"&\s*\(+\s*\(\s*&\s*[A-Za-z_]\w*\s*->")


def main():
    sites = [json.loads(line) for line in open(sys.argv[1])]
    cache = {}
    rows = collections.Counter()
    functions = collections.defaultdict(set)
    freed = set()
    samples = collections.defaultdict(list)
    for site in sites:
        path = f"{sys.argv[2]}/{site['file']}.i"
        if path not in cache:
            try:
                cache[path] = open(path, errors="replace").read().split("\n")
            except OSError:
                cache[path] = []
        lines = cache[path]
        text = lines[site["line"] - 1] if 0 < site["line"] <= len(lines) else ""
        if not GROUP.search(text):
            continue
        outer = bool(OUTER_ADDRESS.search(text))
        key = (site["family"], "outer-address" if outer else "dereferenced")
        rows[key] += 1
        functions[key].add((site["file"], site["function"]))
        if site["freed_alone"]:
            freed.add((site["file"], site["function"]))
        if len(samples[key]) < 4:
            samples[key].append(f"{site['function']}: {text.strip()[:160]}")
    out = {
        "groups": {f"{k[0]}:{k[1]}": v for k, v in sorted(rows.items())},
        "functions": {f"{k[0]}:{k[1]}": len(v) for k, v in sorted(functions.items())},
        "distinct_functions": len(set().union(*functions.values())) if functions else 0,
        "freed_alone": sorted(f"{f}::{n}" for f, n in freed),
        "samples": {f"{k[0]}:{k[1]}": v for k, v in samples.items()},
    }
    json.dump(out, open(sys.argv[3], "w"), indent=1)
    print(json.dumps({k: v for k, v in out.items() if k != "samples"}, indent=1))


if __name__ == "__main__":
    main()
